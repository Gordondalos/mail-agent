#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod gmail;
mod notifier;
mod oauth;

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use config::{Settings, SettingsManager, SettingsUpdate};
use gmail::{wait_for_authorisation, GmailClient, GmailNotification};
use notifier::NotificationQueue;
use oauth::{ensure_autostart, AccessTokenProvider, OAuthController, OAuthError};
use serde::Serialize;
use serde_json;
use tauri::WindowEvent;
use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItem},
    path::BaseDirectory,
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};
use tauri_plugin_autostart::MacosLauncher;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

const AUTH_REQUIRED_MESSAGE: &str =
    "Авторизация в Gmail недоступна. Откройте окно настроек и выполните вход.";
const AUTH_CONFIG_MESSAGE: &str =
    "Укажите OAuth Client ID и выполните авторизацию в настройках, чтобы продолжить.";

#[derive(Clone)]
struct AppState {
    settings: Arc<SettingsManager>,
    oauth: Arc<OAuthController>,
    gmail: Arc<GmailClient>,
    notifier: Arc<NotificationQueue>,
    auth_prompted: Arc<AtomicBool>,
    snooze_until: Arc<Mutex<Option<std::time::Instant>>>,
}

impl AppState {
    async fn poll_once(&self, app: &AppHandle) -> Result<()> {
        if !self.oauth.is_configured() {
            self.prompt_auth_once(app, AUTH_CONFIG_MESSAGE);
            return Ok(());
        }

        if let Some(until) = *self.snooze_until.lock() {
            if std::time::Instant::now() < until {
                debug!("gmail polling snoozed");
                return Ok(());
            } else {
                *self.snooze_until.lock() = None;
            }
        }

        match self
            .gmail
            .fetch_unread(&self.settings.get().gmail_query)
            .await
        {
            Ok(messages) => {
                self.reset_auth_prompt();
                for message in messages {
                    if let Ok(json) = serde_json::to_string(&message) {
                        debug!(notification_json = %json, "gmail: notification payload");
                    }
                    if let Err(err) = self.notifier.enqueue(app, message) {
                        warn!(%err, "failed to enqueue notification");
                    }
                }
            }
            Err(err) => {
                if let Some(kind) = err.downcast_ref::<OAuthError>() {
                    match kind {
                        OAuthError::NotAuthorised => {
                            self.notifier.clear();
                            self.prompt_auth_once(app, AUTH_REQUIRED_MESSAGE);
                            warn!("gmail polling failed: not authorised");
                        }
                        OAuthError::Misconfigured(reason) => {
                            self.prompt_auth_once(app, reason.as_str());
                            warn!(%err, "gmail polling failed");
                        }
                        _ => warn!(%err, "gmail polling failed"),
                    }
                } else {
                    warn!(%err, "gmail polling failed");
                }
            }
        }
        Ok(())
    }

    async fn mark_read(&self, id: &str) -> Result<()> {
        self.gmail.mark_read(id).await
    }

    fn reset_auth_prompt(&self) {
        self.auth_prompted.store(false, Ordering::SeqCst);
    }

    fn prompt_auth_once(&self, app: &AppHandle, message: &str) {
        if self
            .auth_prompted
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            emit_auth_required(app, message);
        }
    }

    fn prompt_auth_force(&self, app: &AppHandle, message: &str) {
        self.auth_prompted.store(true, Ordering::SeqCst);
        emit_auth_required(app, message);
    }
}

#[tauri::command]
async fn initialise(state: tauri::State<'_, AppState>) -> Result<InitialPayload, String> {
    let settings = state.settings.get();
    let provider: Arc<dyn AccessTokenProvider> = state.oauth.clone();
    let authorised = wait_for_authorisation(provider).await;
    Ok(InitialPayload {
        settings,
        authorised,
    })
}

#[derive(serde::Serialize)]
struct InitialPayload {
    settings: Settings,
    authorised: bool,
}

#[tauri::command]
async fn request_authorisation(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let res = state.oauth.authorise(&app).await;
    if let Err(ref err) = res {
        error!(?err, "authorisation failed");
    }
    res.map_err(stringify_error)?;
    state.oauth.load_cached();
    state.poll_once(&app).await.map_err(|err| err.to_string())?;
    // Hide settings window after successful authorisation, leave in tray
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
    Ok(())
}

fn stringify_error<E: std::error::Error>(err: E) -> String {
    use std::fmt::Write as _;
    let mut s = format!("{}", err);
    let mut src = err.source();
    while let Some(e) = src {
        let _ = write!(&mut s, "\nCaused by: {}", e);
        src = e.source();
    }
    s
}

#[tauri::command]
async fn revoke(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.oauth.revoke().await.map_err(|err| err.to_string())?;
    state.notifier.clear();
    Ok(())
}

#[tauri::command]
async fn update_settings(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    update: SettingsUpdate,
) -> Result<Settings, String> {
    let settings = state
        .settings
        .update(update)
        .map_err(|err| err.to_string())?;
    ensure_autostart(&app, settings.auto_launch);
    if let Err(err) = app.emit("gmail://settings", &settings) {
        warn!(%err, "failed to broadcast settings");
    }
    Ok(settings)
}

#[tauri::command]
async fn check_now(app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    if !state.oauth.is_configured() {
        state.prompt_auth_force(&app, AUTH_CONFIG_MESSAGE);
        return Ok(());
    }

    if let Err(err) = state.oauth.access_token().await {
        match err {
            OAuthError::NotAuthorised => {
                state.prompt_auth_force(&app, AUTH_REQUIRED_MESSAGE);
                return Ok(());
            }
            OAuthError::Misconfigured(reason) => {
                state.prompt_auth_force(&app, reason.as_str());
                return Ok(());
            }
            _ => return Err(err.to_string()),
        }
    }

    state.poll_once(&app).await.map_err(|err| err.to_string())
}

#[tauri::command]
async fn mark_message_read(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    message_id: String,
) -> Result<(), String> {
    let notifier = state.notifier.clone();
    state
        .mark_read(&message_id)
        .await
        .map_err(|err| err.to_string())?;
    state.gmail.forget(&message_id);
    notifier
        .complete_current(&app)
        .map_err(|err| err.to_string())?;
    Ok(())
}

#[tauri::command]
async fn open_in_browser(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    url: String,
) -> Result<(), String> {
    state
        .notifier
        .complete_current(&app)
        .map_err(|err| err.to_string())?;
    webbrowser::open(&url)
        .map_err(|err| err.to_string())
        .map(|_| ())
}

#[derive(serde::Serialize)]
struct VoicePreset {
    id: String,
    label: String,
    file_name: String,
    path: String,
}

#[tauri::command]
async fn list_voice_tracks(app: AppHandle) -> Result<Vec<VoicePreset>, String> {
    let voice_dir = resolve_voice_dir(&app).map_err(|err| err.to_string())?;
    let mut presets = Vec::new();
    let entries = fs::read_dir(&voice_dir).map_err(|err| err.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
            continue;
        };
        let ext = ext.to_ascii_lowercase();
        if !matches!(ext.as_str(), "mp3" | "wav" | "ogg" | "m4a") {
            continue;
        }
        let file_name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };
        let label = format_voice_label(
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or(&file_name),
        );
        let relative_path = format!("voice/{}", file_name.replace('\\', "/"));
        presets.push(VoicePreset {
            id: format!("voice-{}", file_name),
            label,
            file_name,
            path: relative_path,
        });
    }
    presets.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));
    Ok(presets)
}

#[tauri::command]
async fn dismiss_notification(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    message_id: Option<String>,
) -> Result<(), String> {
    if let Some(id) = message_id {
        state.gmail.forget(&id);
    }
    state
        .notifier
        .complete_current(&app)
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn snooze(app: AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let duration_mins = state.settings.get().snooze_duration_mins;
    let duration = Duration::from_secs(duration_mins * 60);
    *state.snooze_until.lock() = Some(std::time::Instant::now() + duration);
    state.notifier.clear();
    state
        .notifier
        .complete_current(&app)
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn resolve_voice_dir(app: &AppHandle) -> Result<PathBuf> {
    const DEV_VOICE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../frontend/public/voice");
    if let Ok(path) = app.path().resolve("voice", BaseDirectory::Resource) {
        if path.exists() {
            return Ok(path);
        }
    }
    let dev = PathBuf::from(DEV_VOICE_DIR);
    if dev.exists() {
        return Ok(dev);
    }
    anyhow::bail!("voice assets directory is missing");
}

fn format_voice_label(stem: &str) -> String {
    let cleaned = stem.replace(['_', '-'], " ");
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        return "Встроенный звук".to_string();
    }
    trimmed
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => {
                    let mut capitalised = first.to_uppercase().collect::<String>();
                    capitalised.push_str(chars.as_str());
                    capitalised
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[tauri::command]
async fn current_notification(
    state: tauri::State<'_, AppState>,
) -> Result<Option<GmailNotification>, String> {
    Ok(state.notifier.current())
}

fn register_tray(app: &tauri::App) -> tauri::Result<()> {
    let check_now = MenuItem::with_id(app, "check_now", "Проверить сейчас", true, None::<&str>)?;
    let open_settings = MenuItem::with_id(
        app,
        "open_settings",
        "Открыть настройки",
        true,
        None::<&str>,
    )?;
    let auth = MenuItem::with_id(app, "auth", "Войти в Gmail", true, None::<&str>)?;
    let logout = MenuItem::with_id(app, "logout", "Выйти", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Выйти из приложения", true, None::<&str>)?;

    let menu = MenuBuilder::new(app)
        .item(&check_now)
        .item(&open_settings)
        .separator()
        .item(&auth)
        .item(&logout)
        .separator()
        .item(&quit)
        .build()?;

    let tray_icon = Image::from_path("icons/icon.ico").unwrap_or_else(|_| {
        app.default_window_icon()
            .cloned()
            .expect("missing default icon")
    });

    TrayIconBuilder::new()
        .icon(tray_icon)
        .menu(&menu)
        .tooltip("Gmail Tray Notifier")
        // Оставим меню на правый клик, а левый клик — показать/скрыть окно
        .show_menu_on_left_click(false)
        .on_menu_event(|app_handle, event| match event.id().as_ref() {
            "check_now" => {
                info!("tray click: check_now");
                let poll_handle = app_handle.clone();
                let app_state = poll_handle.state::<AppState>().inner().clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(err) = app_state.poll_once(&poll_handle).await {
                        warn!(%err, "manual poll failed");
                    }
                });
            }
            "open_settings" => {
                info!("tray click: open_settings");
                if let Some(win) = app_handle.get_webview_window("main") {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
            "auth" => {
                info!("tray click: auth");
                let auth_handle = app_handle.clone();
                let app_state = auth_handle.state::<AppState>().inner().clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(err) = app_state.oauth.authorise(&auth_handle).await {
                        warn!(%err, "authorisation from tray failed");
                    }
                });
            }
            "logout" => {
                info!("tray click: logout");
                let app_state = app_handle.state::<AppState>().inner().clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(err) = app_state.oauth.revoke().await {
                        warn!(%err, "failed to revoke tokens");
                    }
                });
            }
            "quit" => {
                info!("tray click: quit");
                app_handle.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| match event {
            TrayIconEvent::Click { button, .. } | TrayIconEvent::DoubleClick { button, .. } => {
                debug!(?button, "tray icon click");
                if button == MouseButton::Left {
                    let handle = tray.app_handle();
                    if let Some(win) = handle.get_webview_window("main") {
                        let _ = win.show();
                        let _ = win.set_focus();
                    }
                }
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}

fn main() {
    // Avoid broken system proxy settings interfering with OAuth HTTPS calls
    // If NO_PROXY isn't set, exclude Google OAuth hosts and localhost from proxying
    let no_proxy_is_set =
        std::env::var_os("NO_PROXY").is_some() || std::env::var_os("no_proxy").is_some();
    if !no_proxy_is_set {
        std::env::set_var(
            "NO_PROXY",
            "127.0.0.1,localhost,oauth2.googleapis.com,accounts.google.com,googleapis.com,google.com",
        );
    }

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        tracing_subscriber::EnvFilter::new("gmail_tray_notifier=debug,reqwest=debug,hyper=debug")
    });
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .init();

    tauri::Builder::default()
        // Перехватываем закрытие окна крестиком: прячем вместо уничтожения
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = window.hide();
            }
            _ => {}
        })
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .setup(|app| {
            let app_handle = app.handle();
            let settings = Arc::new(SettingsManager::initialize(&app_handle)?);
            let oauth = Arc::new(OAuthController::new(settings.clone()));
            oauth.load_cached();
            let token_provider: Arc<dyn AccessTokenProvider> = oauth.clone();
            let gmail = Arc::new(GmailClient::new(token_provider)?);
            let notifier = Arc::new(NotificationQueue::new());

            app.manage(AppState {
                settings: settings.clone(),
                oauth: oauth.clone(),
                gmail: gmail.clone(),
                notifier: notifier.clone(),
                auth_prompted: Arc::new(AtomicBool::new(false)),
                snooze_until: Arc::new(Mutex::new(None)),
            });

            ensure_autostart(&app_handle, settings.get().auto_launch);

            register_tray(app)?;

            // На старте скрываем окно, если уже авторизованы
            let hide_handle = app_handle.clone();
            let provider: Arc<dyn AccessTokenProvider> = oauth.clone();
            tauri::async_runtime::spawn(async move {
                if wait_for_authorisation(provider).await {
                    if let Some(win) = hide_handle.get_webview_window("main") {
                        let _ = win.hide();
                    }
                }
            });

            let app_state = app.state::<AppState>().inner().clone();
            let poll_handle = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    let interval = app_state.settings.get().poll_interval_secs;
                    if let Err(err) = app_state.poll_once(&poll_handle).await {
                        warn!(%err, "polling failed");
                    }
                    sleep(Duration::from_secs(interval.max(15))).await;
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            initialise,
            request_authorisation,
            revoke,
            update_settings,
            check_now,
            mark_message_read,
            open_in_browser,
            dismiss_notification,
            snooze,
            current_notification,
            list_voice_tracks
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[derive(Debug, Clone, Serialize)]
struct AuthPromptPayload {
    message: String,
}

fn emit_auth_required(app: &AppHandle, message: &str) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    }
    if let Err(err) = app.emit(
        "gmail://auth-required",
        &AuthPromptPayload {
            message: message.to_string(),
        },
    ) {
        warn!(%err, "failed to notify auth requirement");
    }
}
