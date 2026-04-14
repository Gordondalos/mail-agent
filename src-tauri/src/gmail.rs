use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE, URL_SAFE_NO_PAD},
    Engine as _,
};
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
        self.fetch_unread_internal(query, 10, true).await
    }

    pub async fn list_unread_preview(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<GmailNotification>> {
        self.fetch_unread_internal(query, limit, false).await
    }

    async fn fetch_unread_internal(
        &self,
        query: &str,
        limit: usize,
        apply_dedup: bool,
    ) -> Result<Vec<GmailNotification>> {
        let token = self.token_provider.access_token().await?;
        let url = format!("{}/messages", GMAIL_API);
        debug!(%url, %query, "gmail: listing messages");
        let max_results = limit.max(1).min(100);
        let response = self
            .http
            .get(url)
            .bearer_auth(token)
            .query(&[
                ("q", query),
                ("maxResults", &max_results.to_string()),
                ("labelIds", "UNREAD"),
            ])
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
        let mut items = list.messages.unwrap_or_default();
        if items.len() > max_results {
            items.truncate(max_results);
        }
        debug!(count = items.len(), "gmail: list parsed");
        for item in items {
            if apply_dedup && self.is_known(&item.id) {
                continue;
            }
            match self.fetch_message(&item.id).await {
                Ok(notification) => {
                    if apply_dedup {
                        self.mark_seen(&notification.id);
                    }
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
            .query(&[("format", "full")])
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
        debug!(
            "fetch_message: получено сообщение id={}, payload.parts.len={}",
            details.id,
            details.payload.parts.len()
        );
        debug!(
            "fetch_message: payload.body.is_some={}",
            details.payload.body.is_some()
        );
        if !details.payload.parts.is_empty() {
            for (i, part) in details.payload.parts.iter().enumerate() {
                debug!(
                    "fetch_message: часть[{}] mime_type={}, body.is_some={}, parts.len={}",
                    i,
                    part.mime_type,
                    part.body.is_some(),
                    part.parts.len()
                );
            }
        }
        // Для некоторых MIME-частей Gmail отдает тело только по attachmentId.
        let extracted_body = self
            .extract_body_with_attachments(id, &details.payload)
            .await;
        Ok(details.into_notification_with_body(extracted_body))
    }

    async fn extract_body_with_attachments(
        &self,
        message_id: &str,
        payload: &MessagePayload,
    ) -> Option<String> {
        if let Some(html) = find_part_by_mime(payload, "text/html") {
            if let Some(mut decoded) = self.decode_or_fetch_body(message_id, &html).await {
                if decoded.contains("cid:") || decoded.contains("CID:") {
                    let inline_resources = self.collect_inline_resources(message_id, payload).await;
                    if !inline_resources.is_empty() {
                        decoded = replace_cid_urls(&decoded, &inline_resources);
                    }
                }
                return Some(decoded);
            }
        }

        if let Some(text) = find_part_by_mime(payload, "text/plain") {
            if let Some(decoded) = self.decode_or_fetch_body(message_id, &text).await {
                return Some(decoded);
            }
        }

        if let Some(body) = payload.body.as_ref() {
            if let Some(decoded) = self.decode_or_fetch_body(message_id, body).await {
                return Some(decoded);
            }
        }

        None
    }

    async fn decode_or_fetch_body(&self, message_id: &str, body: &MessageBody) -> Option<String> {
        if let Some(decoded) = decode_body(body) {
            return Some(decoded);
        }

        let attachment_id = body.attachment_id.as_deref()?;
        self.fetch_attachment_body(message_id, attachment_id).await
    }

    async fn decode_or_fetch_body_bytes(
        &self,
        message_id: &str,
        body: &MessageBody,
    ) -> Option<Vec<u8>> {
        if let Some(decoded) = decode_body_bytes(body) {
            return Some(decoded);
        }

        let attachment_id = body.attachment_id.as_deref()?;
        self.fetch_attachment_bytes(message_id, attachment_id).await
    }

    async fn fetch_attachment_body(&self, message_id: &str, attachment_id: &str) -> Option<String> {
        self.fetch_attachment_bytes(message_id, attachment_id)
            .await
            .and_then(|bytes| String::from_utf8(bytes).ok())
    }

    async fn fetch_attachment_bytes(
        &self,
        message_id: &str,
        attachment_id: &str,
    ) -> Option<Vec<u8>> {
        let token = match self.token_provider.access_token().await {
            Ok(token) => token,
            Err(err) => {
                warn!(%err, %message_id, %attachment_id, "gmail: failed to get token for attachment");
                return None;
            }
        };

        let url = format!(
            "{}/messages/{}/attachments/{}",
            GMAIL_API, message_id, attachment_id
        );
        let response = match self.http.get(url).bearer_auth(token).send().await {
            Ok(resp) => resp,
            Err(err) => {
                warn!(%err, %message_id, %attachment_id, "gmail: attachment request failed");
                return None;
            }
        };

        if !response.status().is_success() {
            warn!(status = %response.status(), %message_id, %attachment_id, "gmail: attachment response status is not success");
            return None;
        }

        let payload: AttachmentPayload = match response.json().await {
            Ok(payload) => payload,
            Err(err) => {
                warn!(%err, %message_id, %attachment_id, "gmail: invalid attachment response");
                return None;
            }
        };

        payload.data.as_deref().and_then(decode_base64url_bytes)
    }

    async fn collect_inline_resources(
        &self,
        message_id: &str,
        payload: &MessagePayload,
    ) -> HashMap<String, String> {
        let mut candidates = Vec::new();
        collect_inline_candidates(&payload.parts, &mut candidates);
        let mut resources = HashMap::new();

        for candidate in candidates {
            if resources.contains_key(&candidate.cid) {
                continue;
            }
            let Some(bytes) = self
                .decode_or_fetch_body_bytes(message_id, &candidate.body)
                .await
            else {
                continue;
            };
            let mime_type = candidate
                .mime_type
                .split(';')
                .next()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("application/octet-stream");
            let data_url = format!("data:{};base64,{}", mime_type, STANDARD.encode(bytes));
            resources.insert(candidate.cid, data_url);
        }

        resources
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
    #[serde(default)]
    headers: Vec<Header>,
    body: Option<MessageBody>,
    #[serde(default)]
    parts: Vec<MessagePart>,
}

#[derive(Debug, Clone, Deserialize)]
struct MessageBody {
    data: Option<String>,
    #[serde(rename = "attachmentId")]
    attachment_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct Header {
    name: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct AttachmentPayload {
    data: Option<String>,
}

#[derive(Debug, Clone)]
struct InlinePartCandidate {
    cid: String,
    mime_type: String,
    body: MessageBody,
}

impl Message {
    fn into_notification(self) -> GmailNotification {
        let body = extract_body(&self.payload);
        self.into_notification_with_body(body)
    }

    fn into_notification_with_body(self, body: Option<String>) -> GmailNotification {
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
            message = self.id.clone()
        );

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
        debug!(
            "extract_body: часть[{}]: mime_type={}, has_body={}, has_parts={}",
            i,
            part.mime_type,
            part.body.is_some(),
            !part.parts.is_empty()
        );
    }

    // Сначала пытаемся найти HTML версию
    if let Some(html) = find_part_by_mime(payload, "text/html") {
        debug!("extract_body: найден HTML");
        let decoded = decode_body(&html);
        debug!(
            "extract_body: HTML декодирован, результат = {}",
            decoded.is_some()
        );
        if decoded.is_some() {
            return decoded;
        }
    }

    // Затем пытаемся найти plain text
    if let Some(text) = find_part_by_mime(payload, "text/plain") {
        debug!("extract_body: найден plain text");
        let decoded = decode_body(&text);
        debug!(
            "extract_body: plain text декодирован, результат = {}",
            decoded.is_some()
        );
        if decoded.is_some() {
            return decoded;
        }
    }

    // Если нет частей, проверяем body непосредственно в payload
    if let Some(ref body) = payload.body {
        debug!("extract_body: используем body из payload");
        let decoded = decode_body(body);
        debug!(
            "extract_body: body декодирован, результат = {}",
            decoded.is_some()
        );
        if decoded.is_some() {
            return decoded;
        }
    }

    debug!("extract_body: тело письма не найдено");
    None
}

fn find_part_by_mime(payload: &MessagePayload, mime_type: &str) -> Option<MessageBody> {
    debug!("find_part_by_mime: ищем тип {}", mime_type);
    let target = mime_type.to_ascii_lowercase();
    // Рекурсивный поиск по частям
    for part in &payload.parts {
        debug!(
            "find_part_by_mime: проверяем часть с типом {}",
            part.mime_type
        );
        let part_mime = part.mime_type.to_ascii_lowercase();

        // Если это multipart контейнер, сначала проверяем его подчасти
        if part_mime.starts_with("multipart/") {
            debug!(
                "find_part_by_mime: найден multipart контейнер, проверяем {} вложенных частей",
                part.parts.len()
            );
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
        if part_mime == target || part_mime.starts_with(&(target.clone() + ";")) {
            if let Some(ref body) = part.body {
                let has_data = body.data.as_ref().is_some_and(|d| !d.is_empty());
                let has_attachment = body.attachment_id.as_ref().is_some_and(|id| !id.is_empty());
                debug!(
                    "find_part_by_mime: найдено тело для типа {}, has_data={}, has_attachment={}",
                    mime_type, has_data, has_attachment
                );
                if has_data || has_attachment {
                    return Some(body.clone());
                }
            } else {
                debug!("find_part_by_mime: у части с типом {} нет body", mime_type);
            }
        }

        // Рекурсивный поиск во вложенных частях
        if !part.parts.is_empty() {
            debug!(
                "find_part_by_mime: у части есть {} вложенных частей",
                part.parts.len()
            );
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
    debug!(
        "decode_body: начало декодирования, есть ли data = {}",
        body.data.is_some()
    );
    body.data.as_deref().and_then(decode_base64url_text)
}

fn decode_body_bytes(body: &MessageBody) -> Option<Vec<u8>> {
    body.data.as_deref().and_then(decode_base64url_bytes)
}

fn decode_base64url_text(data: &str) -> Option<String> {
    let decoded = decode_base64url_bytes(data).and_then(|bytes| {
        debug!("decode_body: декодировано {} байт", bytes.len());
        String::from_utf8(bytes).ok()
    });
    debug!(
        "decode_body: результат декодирования = {}",
        decoded.is_some()
    );
    decoded
}

fn decode_base64url_bytes(data: &str) -> Option<Vec<u8>> {
    // Иногда Gmail возвращает переносы строк внутри data.
    let compact: String = data.chars().filter(|c| !c.is_whitespace()).collect();
    URL_SAFE_NO_PAD
        .decode(compact.as_bytes())
        .or_else(|_| URL_SAFE.decode(compact.as_bytes()))
        .ok()
}

fn collect_inline_candidates(parts: &[MessagePart], out: &mut Vec<InlinePartCandidate>) {
    for part in parts {
        if let Some(body) = part.body.as_ref() {
            let has_data = body.data.as_ref().is_some_and(|value| !value.is_empty());
            let has_attachment = body
                .attachment_id
                .as_ref()
                .is_some_and(|value| !value.is_empty());
            if (has_data || has_attachment)
                && !part
                    .mime_type
                    .to_ascii_lowercase()
                    .starts_with("multipart/")
            {
                if let Some(cid) = extract_content_id(&part.headers) {
                    out.push(InlinePartCandidate {
                        cid,
                        mime_type: part.mime_type.clone(),
                        body: body.clone(),
                    });
                }
            }
        }

        if !part.parts.is_empty() {
            collect_inline_candidates(&part.parts, out);
        }
    }
}

fn extract_content_id(headers: &[Header]) -> Option<String> {
    headers
        .iter()
        .find(|header| {
            header.name.eq_ignore_ascii_case("Content-ID")
                || header.name.eq_ignore_ascii_case("X-Attachment-Id")
        })
        .and_then(|header| normalize_content_id(&header.value))
}

fn normalize_content_id(value: &str) -> Option<String> {
    let trimmed = value
        .trim()
        .trim_matches('<')
        .trim_matches('>')
        .trim_matches('"');
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_ascii_lowercase())
}

fn replace_cid_urls(html: &str, inline_resources: &HashMap<String, String>) -> String {
    if inline_resources.is_empty() {
        return html.to_string();
    }

    let lower = html.to_ascii_lowercase();
    let bytes = html.as_bytes();
    let mut rewritten = String::with_capacity(html.len());
    let mut cursor = 0usize;
    let mut search_from = 0usize;

    while let Some(rel_pos) = lower[search_from..].find("cid:") {
        let start = search_from + rel_pos;
        rewritten.push_str(&html[cursor..start]);

        let mut end = start + 4;
        while end < bytes.len() {
            let ch = bytes[end];
            if matches!(
                ch,
                b'"' | b'\'' | b' ' | b'\t' | b'\r' | b'\n' | b')' | b'>'
            ) {
                break;
            }
            end += 1;
        }

        let raw_cid = &html[start + 4..end];
        let normalized = normalize_content_id(raw_cid);
        if let Some(data_url) = normalized.and_then(|cid| inline_resources.get(&cid).cloned()) {
            rewritten.push_str(&data_url);
        } else {
            rewritten.push_str(&html[start..end]);
        }

        cursor = end;
        search_from = end;
    }

    rewritten.push_str(&html[cursor..]);
    rewritten
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
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use std::collections::HashMap;

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

    #[test]
    fn decodes_gmail_base64url_without_padding() {
        let plain = "Hello from Gmail body";
        let encoded = URL_SAFE_NO_PAD.encode(plain.as_bytes());
        let body = MessageBody {
            data: Some(encoded),
            attachment_id: None,
        };
        assert_eq!(decode_body(&body).as_deref(), Some(plain));
    }

    #[test]
    fn finds_mime_with_charset_parameters() {
        let plain = "text body";
        let encoded = URL_SAFE_NO_PAD.encode(plain.as_bytes());
        let payload = MessagePayload {
            headers: vec![],
            parts: vec![MessagePart {
                mime_type: "text/plain; charset=UTF-8".to_string(),
                headers: vec![],
                body: Some(MessageBody {
                    data: Some(encoded),
                    attachment_id: None,
                }),
                parts: vec![],
            }],
            body: None,
        };

        let found = find_part_by_mime(&payload, "text/plain").expect("part found");
        assert_eq!(decode_body(&found).as_deref(), Some(plain));
    }

    #[test]
    fn finds_part_with_attachment_id_even_without_inline_data() {
        let payload = MessagePayload {
            headers: vec![],
            parts: vec![MessagePart {
                mime_type: "text/html; charset=UTF-8".to_string(),
                headers: vec![],
                body: Some(MessageBody {
                    data: None,
                    attachment_id: Some("att-1".to_string()),
                }),
                parts: vec![],
            }],
            body: None,
        };

        let found = find_part_by_mime(&payload, "text/html").expect("part found");
        assert_eq!(found.attachment_id.as_deref(), Some("att-1"));
    }

    #[test]
    fn decodes_base64url_with_whitespace() {
        let plain = "Long body with spacing";
        let encoded = URL_SAFE_NO_PAD.encode(plain.as_bytes());
        let spaced = format!("{}\n{}", &encoded[..6], &encoded[6..]);
        let body = MessageBody {
            data: Some(spaced),
            attachment_id: None,
        };
        assert_eq!(decode_body(&body).as_deref(), Some(plain));
    }

    #[test]
    fn replaces_cid_urls_with_data_urls() {
        let html = r#"<img src="cid:jira-generated-image-avatar-123">"#;
        let mut inline = HashMap::new();
        inline.insert(
            "jira-generated-image-avatar-123".to_string(),
            "data:image/png;base64,AAA".to_string(),
        );
        let replaced = replace_cid_urls(html, &inline);
        assert!(replaced.contains("data:image/png;base64,AAA"));
        assert!(!replaced.contains("cid:jira-generated-image-avatar-123"));
    }

    #[test]
    fn normalizes_content_id_wrappers() {
        assert_eq!(
            normalize_content_id("<CID-Value@Mail>").as_deref(),
            Some("cid-value@mail")
        );
    }
}
