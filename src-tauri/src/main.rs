#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod gmail;
mod notifier;
mod oauth;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use config::{Settings, SettingsManager, SettingsUpdate};
use gmail::{wait_for_authorisation, GmailClient};
use notifier::NotificationQueue;
use oauth::{ensure_autostart, AccessTokenProvider, OAuthController, OAuthError};
use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};
use tauri_plugin_autostart::MacosLauncher;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

#[derive(Clone)]
struct AppState {
    settings: Arc<SettingsManager>,
    oauth: Arc<OAuthController>,
    gmail: Arc<GmailClient>,
    notifier: Arc<NotificationQueue>,
}

impl AppState {
    async fn poll_once(&self, app: &AppHandle) -> Result<()> {
        if !self.oauth.is_configured() {
            return Ok(());
        }
        match self
            .gmail
            .fetch_unread(&self.settings.get().gmail_query)
            .await
        {
            Ok(messages) => {
                for message in messages {
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
                            warn!("gmail polling failed: not authorised");
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

#[tauri::command]
async fn dismiss_notification(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state
        .notifier
        .complete_current(&app)
        .map_err(|err| err.to_string())
}

fn register_tray(app: &tauri::App) -> tauri::Result<()> {
    let check_now = MenuItem::with_id(app, "check_now", "Проверить сейчас", true, None::<&str>)?;
    let open_settings = MenuItem::with_id(app, "open_settings", "Открыть настройки", true, None::<&str>)?;
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

    let tray_icon = Image::from_path("icons/icon.ico")
        .unwrap_or_else(|_| app.default_window_icon().cloned().expect("missing default icon"));

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
                        if win.is_visible().unwrap_or(false) {
                            let _ = win.hide();
                        } else {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
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
    let no_proxy_is_set = std::env::var_os("NO_PROXY").is_some() || std::env::var_os("no_proxy").is_some();
    if !no_proxy_is_set {
        std::env::set_var(
            "NO_PROXY",
            "127.0.0.1,localhost,oauth2.googleapis.com,accounts.google.com,googleapis.com,google.com",
        );
    }

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(
            "gmail_tray_notifier=debug,reqwest=debug,hyper=debug",
        ));
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .init();

    tauri::Builder::default()
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
            dismiss_notification
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
