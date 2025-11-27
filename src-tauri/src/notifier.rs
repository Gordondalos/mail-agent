use std::collections::VecDeque;

use anyhow::Result;
use parking_lot::Mutex;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};
use tracing::{info, warn};

use crate::{config::Settings, gmail::GmailNotification};

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

    pub fn enqueue(&self, app: &AppHandle, notification: GmailNotification, settings: &Settings) -> Result<()> {
        info!("notifier.enqueue: получено уведомление {}", notification.id);
        let mut state = self.inner.lock();
        if state.current.is_none() {
            info!("notifier.enqueue: текущего уведомления нет, показываем сразу");
            state.current = Some(notification.clone());
            drop(state);
            emit_notification(app, notification, settings)?;
        } else {
            info!("notifier.enqueue: есть активное ��ведомление, кладём в очередь ({} элементов)", state.pending.len());
            state.pending.push_back(notification);
        }
        Ok(())
    }

    pub fn current(&self) -> Option<GmailNotification> {
        self.inner.lock().current.clone()
    }

    pub fn replay_current(&self, app: &AppHandle, settings: &Settings) -> Result<bool> {
        info!("notifier.replay_current: пытаемся показать текущее уведомление заново");
        let current = self.inner.lock().current.clone();
        match current {
            Some(notification) => {
                info!("notifier.replay_current: найдено {} — запускаем emit_notification", notification.id);
                emit_notification(app, notification, settings)?;
                Ok(true)
            }
            None => {
                info!("notifier.replay_current: активного уведомления нет");
                Ok(false)
            }
        }
    }

    pub fn complete_current(&self, app: &AppHandle, settings: &Settings) -> Result<()> {
        info!("notifier.complete_current: завершаем текущее уведомление");
        let mut state = self.inner.lock();
        state.current = None;
        if let Some(next) = state.pending.pop_front() {
            info!("notifier.complete_current: берём следующее уведомление {}", next.id);
            state.current = Some(next.clone());
            drop(state);
            emit_notification(app, next, settings)?;
        } else if let Some(win) = app.get_webview_window("alert") {
            info!("notifier.complete_current: очередь пуста, скрываем окно");
            let _ = win.hide();
        }
        Ok(())
    }

    pub fn clear(&self) {
        info!("notifier.clear: очищаем все уведомления");
        let mut state = self.inner.lock();
        state.current = None;
        state.pending.clear();
    }
}

fn emit_notification(app: &AppHandle, notification: GmailNotification, settings: &Settings) -> Result<()> {
    info!("emit_notification: выводим уведомление {}", notification.id);
    if let Some(win) = app.get_webview_window("alert") {
        info!("emit_notification: настраиваем окно уведомления");
        // Применяем размеры из настроек
        let _ = win.set_size(PhysicalSize::new(
            settings.notification_width,
            settings.notification_height,
        ));
        place_alert_window(&win);
        let _ = win.show();
        let _ = win.set_focus();
    } else {
        warn!("emit_notification: окно alert не найдено");
    }
    app.emit("gmail://notification", &notification)?;
    Ok(())
}

const ALERT_MARGIN: i32 = 64;

fn place_alert_window(win: &WebviewWindow) {
    let Ok(size) = win.outer_size() else { return };
    let monitor = win
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| win.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else { return };

    let monitor_pos = monitor.position();
    let monitor_size = monitor.size();

    let width = size.width as i32;
    let height = size.height as i32;
    let x = monitor_pos.x + monitor_size.width as i32 - width - ALERT_MARGIN;
    let y = monitor_pos.y + monitor_size.height as i32 - height - ALERT_MARGIN;

    let _ = win.set_position(PhysicalPosition::new(
        x.max(monitor_pos.x),
        y.max(monitor_pos.y),
    ));
}

#[derive(Debug, Clone, Serialize)]
pub struct NotificationResult {
    pub handled: bool,
}

pub fn notify_dismissed(app: &AppHandle, handled: bool) {
    if let Err(err) = app.emit(
        "gmail://notification-complete",
        &NotificationResult { handled },
    ) {
        warn!(%err, "failed to notify dismissal");
    }
}
