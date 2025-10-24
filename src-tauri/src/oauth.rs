use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{
    AccessToken, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, RefreshToken, Scope, TokenResponse, TokenUrl,
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::api::shell;
use tauri::AppHandle;
use tokio::net::TcpListener;
use tokio::sync::OnceCell;
use tokio::time::timeout;
use tracing::warn;

use crate::config::SettingsManager;

const OAUTH_REDIRECT_PORT: u16 = 42813;
const SERVICE_NAME: &str = "gmail_tray_notifier";
const TOKEN_USER: &str = "gmail";

#[derive(thiserror::Error, Debug)]
pub enum OAuthError {
    #[error("not authorised")]
    NotAuthorised,
    #[error("oauth misconfigured: {0}")]
    Misconfigured(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[async_trait]
pub trait AccessTokenProvider: Send + Sync {
    async fn access_token(&self) -> Result<String, OAuthError>;
    async fn revoke(&self) -> Result<(), OAuthError>;
    fn is_configured(&self) -> bool;
}

#[derive(Clone)]
pub struct OAuthController {
    settings: Arc<SettingsManager>,
    client: OnceCell<BasicClient>,
    cache: Arc<Mutex<Option<TokenSet>>>,
    storage: TokenStorage,
}

impl OAuthController {
    pub fn new(settings: Arc<SettingsManager>) -> Self {
        Self {
            settings,
            client: OnceCell::new(),
            cache: Arc::new(Mutex::new(None)),
            storage: TokenStorage::new(),
        }
    }

    async fn ensure_client(&self) -> Result<&BasicClient, OAuthError> {
        self.client
            .get_or_try_init(|| async {
                let settings = self.settings.get();
                if settings.oauth_client_id.trim().is_empty() {
                    return Err(OAuthError::Misconfigured(
                        "OAuth client ID is missing. Please update settings.".into(),
                    ));
                }
                let client_id = ClientId::new(settings.oauth_client_id.clone());
                let secret = settings.oauth_client_secret.clone().map(ClientSecret::new);
                let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".into())
                    .map_err(|err| OAuthError::Other(err.into()))?;
                let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".into())
                    .map_err(|err| OAuthError::Other(err.into()))?;
                let redirect_url = RedirectUrl::new(format!(
                    "http://localhost:{}/oauth2callback",
                    OAUTH_REDIRECT_PORT
                ))
                .map_err(|err| OAuthError::Other(err.into()))?;

                let mut client = BasicClient::new(client_id, secret, auth_url, Some(token_url))
                    .set_redirect_uri(redirect_url);
                // Gmail desktop apps require offline access for refresh tokens
                client = client.add_scope(Scope::new(
                    "https://www.googleapis.com/auth/gmail.modify".into(),
                ));
                Ok(client)
            })
            .await
    }

    pub async fn authorise(&self, app: &AppHandle) -> Result<AuthorisationResult, OAuthError> {
        let client = self.ensure_client().await?;
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let (auth_url, csrf_state) = client
            .authorize_url(CsrfToken::new_random)
            .set_pkce_challenge(pkce_challenge)
            .add_scope(Scope::new(
                "https://www.googleapis.com/auth/gmail.modify".into(),
            ))
            .add_scope(Scope::new(
                "https://www.googleapis.com/auth/userinfo.email".into(),
            ))
            .url();

        shell::open(&app.shell_scope(), auth_url.to_string(), None)
            .map_err(|err| OAuthError::Other(err.into()))?;

        let code = wait_for_code(csrf_state.secret()).await?;
        let token_response = client
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(pkce_verifier)
            .request_async(async_http_client)
            .await
            .map_err(|err| OAuthError::Other(err.into()))?;

        let token_set = TokenSet::from_response(&token_response)?;
        self.storage.store(&token_set)?;
        {
            let mut cache = self.cache.lock();
            *cache = Some(token_set.clone());
        }
        Ok(AuthorisationResult {})
    }

    pub fn load_cached(&self) {
        if let Ok(stored) = self.storage.load() {
            let mut cache = self.cache.lock();
            *cache = Some(stored);
        }
    }

    async fn refresh_if_needed(&self, client: &BasicClient) -> Result<Option<String>, OAuthError> {
        let mut cache = self.cache.lock();
        if let Some(token) = cache.as_mut() {
            if !token.is_expired() {
                return Ok(Some(token.access_token.clone()));
            }
            if let Some(refresh) = &token.refresh_token {
                drop(cache);
                let response = client
                    .exchange_refresh_token(&RefreshToken::new(refresh.clone()))
                    .request_async(async_http_client)
                    .await
                    .map_err(|err| OAuthError::Other(err.into()))?;
                let mut cache = self.cache.lock();
                let new_token = TokenSet::from_response(&response)?;
                self.storage.store(&new_token)?;
                *cache = Some(new_token.clone());
                return Ok(Some(new_token.access_token));
            }
        }
        Ok(None)
    }

    async fn token_from_storage(&self) -> Result<Option<String>, OAuthError> {
        let client = match self.ensure_client().await {
            Ok(client) => client,
            Err(err) => return Err(err),
        };
        if let Some(token) = self.refresh_if_needed(client).await? {
            return Ok(Some(token));
        }
        let mut cache = self.cache.lock();
        if let Some(token) = cache.clone() {
            return Ok(Some(token.access_token));
        }
        drop(cache);
        if let Ok(stored) = self.storage.load() {
            let token = stored.access_token.clone();
            {
                let mut cache = self.cache.lock();
                *cache = Some(stored);
            }
            return Ok(Some(token));
        }
        Err(OAuthError::NotAuthorised)
    }
}

#[async_trait]
impl AccessTokenProvider for OAuthController {
    async fn access_token(&self) -> Result<String, OAuthError> {
        let client = self.ensure_client().await?;
        if let Some(token) = self.refresh_if_needed(client).await? {
            return Ok(token);
        }
        let cache = self.cache.lock();
        if let Some(token) = cache.clone() {
            if !token.is_expired() {
                return Ok(token.access_token);
            }
        }
        drop(cache);
        self.token_from_storage()
            .await?
            .ok_or(OAuthError::NotAuthorised)
    }

    async fn revoke(&self) -> Result<(), OAuthError> {
        self.storage.clear()?;
        let mut cache = self.cache.lock();
        *cache = None;
        Ok(())
    }

    fn is_configured(&self) -> bool {
        let settings = self.settings.get();
        !settings.oauth_client_id.trim().is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthorisationResult {}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenSet {
    access_token: String,
    refresh_token: Option<String>,
    expires_at: Option<DateTime<Utc>>,
}

impl TokenSet {
    fn from_response(
        response: &dyn TokenResponse<AccessToken = AccessToken, RefreshToken = RefreshToken>,
    ) -> Result<Self> {
        let access_token = response.access_token().secret().to_string();
        let refresh_token = response
            .refresh_token()
            .map(|token| token.secret().to_string());
        let expires_at = response.expires_in().map(|duration| {
            Utc::now()
                + chrono::Duration::from_std(duration)
                    .unwrap_or_else(|_| chrono::Duration::seconds(3600))
        });
        Ok(Self {
            access_token,
            refresh_token,
            expires_at,
        })
    }

    fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            expires_at < Utc::now() + chrono::Duration::seconds(60)
        } else {
            false
        }
    }
}

struct TokenStorage;

impl TokenStorage {
    fn new() -> Self {
        Self
    }

    fn store(&self, tokens: &TokenSet) -> Result<(), OAuthError> {
        let entry = keyring::Entry::new(SERVICE_NAME, TOKEN_USER)
            .map_err(|err| OAuthError::Other(err.into()))?;
        let serialized =
            serde_json::to_string(tokens).map_err(|err| OAuthError::Other(err.into()))?;
        entry
            .set_password(&serialized)
            .map_err(|err| OAuthError::Other(err.into()))?;
        Ok(())
    }

    fn load(&self) -> Result<TokenSet, OAuthError> {
        let entry = keyring::Entry::new(SERVICE_NAME, TOKEN_USER)
            .map_err(|err| OAuthError::Other(err.into()))?;
        let serialized = entry
            .get_password()
            .map_err(|_| OAuthError::NotAuthorised)?;
        let tokens =
            serde_json::from_str(&serialized).map_err(|err| OAuthError::Other(err.into()))?;
        Ok(tokens)
    }

    fn clear(&self) -> Result<(), OAuthError> {
        let entry = keyring::Entry::new(SERVICE_NAME, TOKEN_USER)
            .map_err(|err| OAuthError::Other(err.into()))?;
        match entry.delete_password() {
            Ok(_) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(OAuthError::Other(err.into())),
        }
    }
}

async fn wait_for_code(expected_state: &str) -> Result<String, OAuthError> {
    let listener = TcpListener::bind(("127.0.0.1", OAUTH_REDIRECT_PORT))
        .await
        .map_err(|err| OAuthError::Other(err.into()))?;

    let (mut stream, _) = timeout(Duration::from_secs(180), listener.accept())
        .await
        .map_err(|_| OAuthError::Other(anyhow!("Timed out waiting for OAuth response")))?
        .map_err(|err| OAuthError::Other(err.into()))?;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut buffer = [0u8; 4096];
    let mut received = Vec::new();
    let bytes = stream
        .read(&mut buffer)
        .await
        .map_err(|err| OAuthError::Other(err.into()))?;
    received.extend_from_slice(&buffer[..bytes]);
    let request = String::from_utf8_lossy(&received);

    let mut code = None;
    let mut state = None;
    if let Some(line) = request.lines().next() {
        if let Some(path) = line.split_whitespace().nth(1) {
            if let Some((_, query)) = path.split_once('?') {
                for pair in query.split('&') {
                    let mut parts = pair.splitn(2, '=');
                    let key = parts.next().unwrap_or("");
                    let value = parts
                        .next()
                        .map(|v| {
                            urlencoding::decode(v)
                                .unwrap_or_else(|_| v.into())
                                .to_string()
                        })
                        .unwrap_or_default();
                    match key {
                        "code" => code = Some(value),
                        "state" => state = Some(value),
                        _ => {}
                    }
                }
            }
        }
    }

    let state_ok = state.as_deref() == Some(expected_state);
    let status_line = if state_ok && code.is_some() {
        "HTTP/1.1 200 OK"
    } else {
        "HTTP/1.1 400 Bad Request"
    };
    let body = if state_ok {
        "<html><body><h1>You may close this window.</h1></body></html>"
    } else {
        "<html><body><h1>Invalid OAuth state.</h1></body></html>"
    };
    let response = format!(
        "{status}\r\ncontent-type: text/html; charset=utf-8\r\ncontent-length: {len}\r\n\r\n{body}",
        status = status_line,
        len = body.len(),
        body = body
    );
    stream
        .write_all(response.as_bytes())
        .await
        .map_err(|err| OAuthError::Other(err.into()))?;

    if !state_ok {
        return Err(OAuthError::Other(anyhow!("OAuth state mismatch")));
    }

    code.ok_or_else(|| OAuthError::Other(anyhow!("Missing authorisation code")))
}

pub fn ensure_autostart(app: &AppHandle, enabled: bool) {
    use tauri_plugin_autostart::ManagerExt;
    let autolaunch = app.autolaunch();
    if enabled {
        if let Err(err) = autolaunch.enable() {
            warn!(%err, "failed to enable autostart");
        }
    } else if let Err(err) = autolaunch.disable() {
        warn!(%err, "failed to disable autostart");
    }
}
