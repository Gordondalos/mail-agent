use std::collections::VecDeque;

use anyhow::Result;
use parking_lot::Mutex;
use serde::Serialize;
use tauri::{AppHandle, Manager};
use tracing::warn;

use crate::gmail::GmailNotification;

#[derive(Default)]
pub struct NotificationQueue {
    inner: Mutex<QueueState>,
}

#[derive(Default)]
struct QueueState {
    current: Option<GmailNotification>,
    pending: VecDeque<GmailNotification>,
}

impl NotificationQueue {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(QueueState::default()),
        }
    }

    pub fn enqueue(&self, app: &AppHandle, notification: GmailNotification) -> Result<()> {
        let mut state = self.inner.lock();
        if state.current.is_none() {
            state.current = Some(notification.clone());
            drop(state);
            emit_notification(app, notification)?;
        } else {
            state.pending.push_back(notification);
        }
        Ok(())
    }

    pub fn complete_current(&self, app: &AppHandle) -> Result<()> {
        let mut state = self.inner.lock();
        state.current = None;
        if let Some(next) = state.pending.pop_front() {
            state.current = Some(next.clone());
            drop(state);
            emit_notification(app, next)?;
        }
        Ok(())
    }

    pub fn clear(&self) {
        let mut state = self.inner.lock();
        state.current = None;
        state.pending.clear();
    }
}

fn emit_notification(app: &AppHandle, notification: GmailNotification) -> Result<()> {
    app.emit_all("gmail://notification", &notification)?;
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct NotificationResult {
    pub handled: bool,
}

pub fn notify_dismissed(app: &AppHandle, handled: bool) {
    if let Err(err) = app.emit_all(
        "gmail://notification-complete",
        &NotificationResult { handled },
    ) {
        warn!(%err, "failed to notify dismissal");
    }
}
