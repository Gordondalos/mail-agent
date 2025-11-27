use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use base64::{Engine as _, engine::general_purpose::STANDARD as base64};
use chrono::{DateTime, Utc};
use lru::LruCache;
use parking_lot::Mutex;
use reqwest::StatusCode;
use serde::Deserialize;
use serde::Serialize;
use tokio::time::timeout;
use tracing::{debug, warn};

use crate::oauth::{AccessTokenProvider, OAuthError};

const GMAIL_API: &str = "https://gmail.googleapis.com/gmail/v1/users/me";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GmailNotification {
    pub id: String,
    pub thread_id: String,
    pub subject: String,
    pub snippet: Option<String>,
    pub sender: Option<String>,
    pub recipient: Option<String>,
    pub received_at: Option<DateTime<Utc>>,
    pub url: String,
    pub body: Option<String>,
}

#[derive(Clone)]
pub struct GmailClient {
    http: reqwest::Client,
    token_provider: Arc<dyn AccessTokenProvider>,
    dedup: Arc<Mutex<LruCache<String, Instant>>>,
}

impl GmailClient {
    pub fn new(token_provider: Arc<dyn AccessTokenProvider>) -> Result<Self> {
        let http = reqwest::Client::builder()
            .use_rustls_tls()
            .timeout(Duration::from_secs(20))
            .build()
            .context("failed to construct HTTP client")?;
        Ok(Self {
            http,
            token_provider,
            dedup: Arc::new(Mutex::new(LruCache::new(
                std::num::NonZeroUsize::new(200).unwrap(),
            ))),
        })
    }

    pub async fn fetch_unread(&self, query: &str) -> Result<Vec<GmailNotification>> {
        let token = self.token_provider.access_token().await?;
        let url = format!("{}/messages", GMAIL_API);
        debug!(%url, %query, "gmail: listing messages");
        let response = self
            .http
            .get(url)
            .bearer_auth(token)
            .query(&[("q", query), ("maxResults", "10"), ("labelIds", "UNREAD")])
            .send()
            .await
            .context("failed to list gmail messages")?;

        debug!(status = ?response.status(), "gmail: list response");
        if response.status() == StatusCode::UNAUTHORIZED {
            return Err(OAuthError::NotAuthorised.into());
        }

        if response.status().is_client_error() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("gmail list returned status {status} body={body}");
        }

        let list: MessageList = response
            .json()
            .await
            .context("invalid gmail list response")?;
        let mut notifications = Vec::new();
        let items = list.messages.unwrap_or_default();
        debug!(count = items.len(), "gmail: list parsed");
        for item in items {
            if self.is_known(&item.id) {
                continue;
            }
            match self.fetch_message(&item.id).await {
                Ok(notification) => {
                    self.mark_seen(&notification.id);
                    notifications.push(notification);
                }
                Err(err) => warn!(%err, message_id = %item.id, "failed to fetch message"),
            }
        }
        notifications.sort_by_key(|n| n.received_at);
        Ok(notifications)
    }

    pub fn forget(&self, id: &str) {
        let mut guard = self.dedup.lock();
        guard.pop(&id.to_string());
    }

    pub fn clear_cache(&self) {
        let mut guard = self.dedup.lock();
        guard.clear();
    }

    async fn fetch_message(&self, id: &str) -> Result<GmailNotification> {
        let token = self.token_provider.access_token().await?;
        let url = format!("{}/messages/{}", GMAIL_API, id);
        debug!(%id, %url, "gmail: fetch message");
        let response = self
            .http
            .get(url)
            .bearer_auth(token)
            .query(&[
                ("format", "full"),
            ])
            .send()
            .await
            .context("failed to fetch gmail message")?;

        debug!(%id, status = ?response.status(), "gmail: message response");
        if response.status() == StatusCode::UNAUTHORIZED {
            return Err(OAuthError::NotAuthorised.into());
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("gmail message returned status {status} body={body}");
        }

        let details: Message = response
            .json()
            .await
            .context("invalid gmail message response")?;
        debug!("fetch_message: получено сообщение id={}, payload.parts.len={}", details.id, details.payload.parts.len());
        debug!("fetch_message: payload.body.is_some={}", details.payload.body.is_some());
        if !details.payload.parts.is_empty() {
            for (i, part) in details.payload.parts.iter().enumerate() {
                debug!("fetch_message: часть[{}] mime_type={}, body.is_some={}, parts.len={}",
                    i, part.mime_type, part.body.is_some(), part.parts.len());
            }
        }
        Ok(details.into_notification())
    }

    pub async fn mark_read(&self, id: &str) -> Result<()> {
        let token = self.token_provider.access_token().await?;
        let url = format!("{}/messages/{}/modify", GMAIL_API, id);
        debug!(%id, %url, "gmail: mark read");
        let response = self
            .http
            .post(url)
            .bearer_auth(token)
            .json(&serde_json::json!({ "removeLabelIds": ["UNREAD"] }))
            .send()
            .await
            .context("failed to modify message")?;

        debug!(%id, status = ?response.status(), "gmail: mark read response");
        if response.status() == StatusCode::UNAUTHORIZED {
            return Err(OAuthError::NotAuthorised.into());
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("gmail modify returned status {status} body={body}");
        }

        Ok(())
    }

    fn is_known(&self, id: &str) -> bool {
        let mut guard = self.dedup.lock();
        guard.get(&id.to_string()).is_some()
    }

    fn mark_seen(&self, id: &str) {
        let mut guard = self.dedup.lock();
        guard.put(id.to_string(), Instant::now());
    }
}

#[derive(Debug, Deserialize)]
struct MessageList {
    messages: Option<Vec<MessageRef>>,
}

#[derive(Debug, Deserialize)]
struct MessageRef {
    id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Message {
    id: String,
    thread_id: String,
    snippet: Option<String>,
    internal_date: Option<String>,
    payload: MessagePayload,
}

#[derive(Debug, Deserialize)]
struct MessagePayload {
    headers: Vec<Header>,
    #[serde(default)]
    parts: Vec<MessagePart>,
    body: Option<MessageBody>,
}

#[derive(Debug, Clone, Deserialize)]
struct MessagePart {
    #[serde(rename = "mimeType")]
    mime_type: String,
    body: Option<MessageBody>,
    #[serde(default)]
    parts: Vec<MessagePart>,
}

#[derive(Debug, Clone, Deserialize)]
struct MessageBody {
    data: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Header {
    name: String,
    value: String,
}

impl Message {
    fn into_notification(self) -> GmailNotification {
        let subject = self
            .payload
            .headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case("Subject"))
            .map(|h| h.value.clone())
            .unwrap_or_else(|| "(без темы)".to_string());
        let sender = self
            .payload
            .headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case("From"))
            .map(|h| h.value.clone());
        let recipient = self
            .payload
            .headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case("To"))
            .map(|h| h.value.clone());
        let received_at = self
            .payload
            .headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case("Date"))
            .and_then(|h| DateTime::parse_from_rfc2822(&h.value).ok())
            .map(|dt| dt.with_timezone(&Utc));
        let url = format!(
            "https://mail.google.com/mail/u/0/#inbox/{message}",
            message = self.id
        );

        // Извлекаем тело письма (приоритет: HTML, затем plain text)
        let body = extract_body(&self.payload);
        debug!("into_notification: тело письма извлечено, есть ли body = {}", body.is_some());
        if let Some(ref b) = body {
            debug!("into_notification: длина тела = {}", b.len());
        }

        GmailNotification {
            id: self.id,
            thread_id: self.thread_id,
            subject,
            snippet: self.snippet,
            sender,
            recipient,
            received_at,
            url,
            body,
        }
    }
}

fn extract_body(payload: &MessagePayload) -> Option<String> {
    debug!("extract_body: начало извлечения тела письма");
    debug!("extract_body: количество частей = {}", payload.parts.len());

    // Логируем структуру частей для отладки
    for (i, part) in payload.parts.iter().enumerate() {
        debug!("extract_body: часть[{}]: mime_type={}, has_body={}, has_parts={}",
            i, part.mime_type, part.body.is_some(), !part.parts.is_empty());
    }

    // Сначала пытаемся найти HTML версию
    if let Some(html) = find_part_by_mime(payload, "text/html") {
        debug!("extract_body: найден HTML");
        let decoded = decode_body(&html);
        debug!("extract_body: HTML декодирован, результат = {}", decoded.is_some());
        if decoded.is_some() {
            return decoded;
        }
    }

    // Затем пытаемся найти plain text
    if let Some(text) = find_part_by_mime(payload, "text/plain") {
        debug!("extract_body: найден plain text");
        let decoded = decode_body(&text);
        debug!("extract_body: plain text декодирован, результат = {}", decoded.is_some());
        if decoded.is_some() {
            return decoded;
        }
    }

    // Если нет частей, проверяем body непосредственно в payload
    if let Some(ref body) = payload.body {
        debug!("extract_body: используем body из payload");
        let decoded = decode_body(body);
        debug!("extract_body: body декодирован, результат = {}", decoded.is_some());
        if decoded.is_some() {
            return decoded;
        }
    }

    debug!("extract_body: тело письма не найдено");
    None
}

fn find_part_by_mime(payload: &MessagePayload, mime_type: &str) -> Option<MessageBody> {
    debug!("find_part_by_mime: ищем тип {}", mime_type);
    // Рекурсивный поиск по частям
    for part in &payload.parts {
        debug!("find_part_by_mime: проверяем часть с типом {}", part.mime_type);

        // Если это multipart контейнер, сначала проверяем его подчасти
        if part.mime_type.starts_with("multipart/") {
            debug!("find_part_by_mime: найден multipart контейнер, проверяем {} вложенных частей", part.parts.len());
            if !part.parts.is_empty() {
                let nested_payload = MessagePayload {
                    headers: vec![],
                    parts: part.parts.clone(),
                    body: None,
                };
                if let Some(body) = find_part_by_mime(&nested_payload, mime_type) {
                    return Some(body);
                }
            }
            continue;
        }

        // Проверяем совпадение типа
        if part.mime_type == mime_type {
            if let Some(ref body) = part.body {
                debug!("find_part_by_mime: найдено тело для типа {}, data.is_some={}", mime_type, body.data.is_some());
                if let Some(ref data) = body.data {
                    debug!("find_part_by_mime: длина data={}", data.len());
                    // Возвращаем только если есть данные
                    if !data.is_empty() {
                        return Some(MessageBody { data: body.data.clone() });
                    }
                }
            } else {
                debug!("find_part_by_mime: у части с типом {} нет body", mime_type);
            }
        }

        // Рекурсивный поиск во вложенных частях
        if !part.parts.is_empty() {
            debug!("find_part_by_mime: у части есть {} вложенных частей", part.parts.len());
            let nested_payload = MessagePayload {
                headers: vec![],
                parts: part.parts.clone(),
                body: None,
            };
            if let Some(body) = find_part_by_mime(&nested_payload, mime_type) {
                return Some(body);
            }
        }
    }

    debug!("find_part_by_mime: тип {} не найден", mime_type);
    None
}

fn decode_body(body: &MessageBody) -> Option<String> {
    debug!("decode_body: начало декодирования, есть ли data = {}", body.data.is_some());
    body.data.as_ref().and_then(|data| {
        debug!("decode_body: длина закодированных данных = {}", data.len());
        // Gmail использует URL-safe Base64
        let replaced = data.replace('-', "+").replace('_', "/");
        let decoded = base64.decode(&replaced)
            .ok()
            .and_then(|bytes| {
                debug!("decode_body: декодировано {} байт", bytes.len());
                String::from_utf8(bytes).ok()
            });
        debug!("decode_body: результат декодирования = {}", decoded.is_some());
        decoded
    })
}

pub async fn wait_for_authorisation(token_provider: Arc<dyn AccessTokenProvider>) -> bool {
    match timeout(Duration::from_secs(2), token_provider.access_token()).await {
        Ok(Ok(_)) => true,
        Ok(Err(err)) => {
            warn!(%err, "unable to obtain token");
            false
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_camel_case_message_payload() {
        let json = r#"
        {
            "id": "abc",
            "threadId": "thread-1",
            "snippet": "hello",
            "internalDate": "169",
            "payload": {
                "headers": [
                    { "name": "Subject", "value": "Test subject" },
                    { "name": "From", "value": "Sender <sender@example.com>" },
                    { "name": "Date", "value": "Thu, 24 Oct 2024 15:30:00 +0000" }
                ]
            }
        }
        "#;

        let message: Message = serde_json::from_str(json).expect("message parses");
        assert_eq!(message.id, "abc");
        assert_eq!(message.thread_id, "thread-1");
        assert_eq!(message.snippet.as_deref(), Some("hello"));

        let notification = message.into_notification();
        assert_eq!(notification.subject, "Test subject");
        assert_eq!(
            notification.sender.as_deref(),
            Some("Sender <sender@example.com>")
        );
        assert_eq!(notification.thread_id, "thread-1");
        assert!(notification.received_at.is_some(), "date header converted");
    }
}
