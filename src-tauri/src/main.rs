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
    menu::{MenuBuilder, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};
use tauri_plugin_autostart::MacosLauncher;
use tokio::time::sleep;
use tracing::warn;

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
    state
        .oauth
        .authorise(&app)
        .await
        .map_err(|err| format!("{err:?}"))?;
    state.oauth.load_cached();
    state.poll_once(&app).await.map_err(|err| err.to_string())?;
    Ok(())
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
    let auth = MenuItem::with_id(app, "auth", "Войти в Gmail", true, None::<&str>)?;
    let logout = MenuItem::with_id(app, "logout", "Выйти", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Выйти из приложения", true, None::<&str>)?;

    let menu = MenuBuilder::new(app)
        .item(&check_now)
        .separator()
        .item(&auth)
        .item(&logout)
        .separator()
        .item(&quit)
        .build()?;

    TrayIconBuilder::new()
        .menu(&menu)
        .on_menu_event(|app_handle, event| match event.id().as_ref() {
            "check_now" => {
                let poll_handle = app_handle.clone();
                let app_state = poll_handle.state::<AppState>().inner().clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(err) = app_state.poll_once(&poll_handle).await {
                        warn!(%err, "manual poll failed");
                    }
                });
            }
            "auth" => {
                let auth_handle = app_handle.clone();
                let app_state = auth_handle.state::<AppState>().inner().clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(err) = app_state.oauth.authorise(&auth_handle).await {
                        warn!(%err, "authorisation from tray failed");
                    }
                });
            }
            "logout" => {
                let app_state = app_handle.state::<AppState>().inner().clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(err) = app_state.oauth.revoke().await {
                        warn!(%err, "failed to revoke tokens");
                    }
                });
            }
            "quit" => {
                app_handle.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
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
