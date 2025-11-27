# Исправление проблемы с функцией "Отложить" (v2)

## Проблема
При нажатии кнопки "Отложить" приложение переставало проверять почту даже при нажатии кнопки "Проверить сейчас" в трее или в настройках. Уведомления не появлялись после нажатия "Проверить сейчас" во время режима отложения.

## Обнаруженные причины

### Основная причина (КРИТИЧЕСКАЯ)
**Обработчик трея для "Проверить сейчас" вызывал напрямую `poll_once()` вместо команды `check_now()`**

Это было самой критической проблемой, из-за которой при нажатии в трее режим snooze не сбрасывался.

### Дополнительные причины
1. Функция `snooze()` устанавливала таймер `snooze_until` и очищала очередь уведомлений
2. Функция `check_now()` изначально не сбрасывала таймер snooze
3. При snooze терялась информация о непрочитанных письмах
4. Письма помечались как "seen" в кэше Gmail, поэтому не показывались повторно

## Решение

### 1. ⚠️ КРИТИЧЕСКОЕ: Исправлен обработчик трея (строка ~443)

**Было:**
```rust
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
```

**Стало:**
```rust
"check_now" => {
    info!("tray click: check_now");
    let app_clone = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        // Вызываем команду check_now, которая правильно обрабатывает snooze
        if let Err(err) = check_now(app_clone.clone(), app_clone.state()).await {
            warn!(%err, "manual check failed");
        }
    });
}
```

### 2. Изменена логика функции `snooze()` (строка ~342)
Теперь функция **НЕ** очищает очередь уведомлений, а только скрывает окно:

```rust
#[tauri::command]
async fn snooze(app: AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let duration_mins = state.settings.get().snooze_duration_mins;
    let duration = Duration::from_secs(duration_mins * 60);
    *state.snooze_until.lock() = Some(std::time::Instant::now() + duration);
    
    // Скрываем окно уведомления, но не очищаем очередь
    // Уведомления появятся снова после окончания snooze
    if let Some(win) = app.get_webview_window("alert") {
        let _ = win.hide();
    }
    Ok(())
}
```

### 3. Улучшена функция `check_now()` (строка ~207)
Добавлен сброс snooze, логирование и показ текущего уведомления:

```rust
#[tauri::command]
async fn check_now(app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    info!("check_now: manual check requested");
    
    // ...проверки авторизации...
    
    // Сбрасываем режим отложения при принудительной проверке
    let was_snoozed = state.snooze_until.lock().is_some();
    *state.snooze_until.lock() = None;
    if was_snoozed {
        info!("check_now: snooze cleared");
    }

    // Показываем текущее уведомление если оно было скрыто при snooze
    if let Some(notification) = state.notifier.current() {
        info!("check_now: showing current notification from queue");
        if let Some(win) = app.get_webview_window("alert") {
            let _ = win.show();
            let _ = win.set_focus();
            let _ = app.emit("gmail://notification", &notification);
        }
    } else {
        info!("check_now: no notification in queue");
    }

    info!("check_now: calling poll_once");
    state.poll_once(&app).await.map_err(|err| err.to_string())
}
```

### 4. Улучшена функция `poll_once()` (строка ~51)
После истечения времени отложения автоматически показывается текущее уведомление:

```rust
if let Some(until) = *self.snooze_until.lock() {
    if std::time::Instant::now() < until {
        debug!("gmail polling snoozed");
        return Ok(());
    } else {
        *self.snooze_until.lock() = None;
        // После окончания snooze, показываем текущее уведомление если оно есть
        if let Some(notification) = self.notifier.current() {
            if let Some(win) = app.get_webview_window("alert") {
                let _ = win.show();
                let _ = win.set_focus();
                let _ = app.emit("gmail://notification", &notification);
            }
        }
    }
}
```

## Изменённые файлы
- `src-tauri/src/main.rs` - изменены функции:
  - `snooze()` - не очищает очередь
  - `check_now()` - сбрасывает snooze и показывает уведомления
  - `poll_once()` - показывает уведомления после окончания snooze
  - **Обработчик меню трея** - вызывает `check_now()` вместо `poll_once()`

## Как теперь работает

### При нажатии "Отложить":
- ✅ Окно уведомления скрывается
- ✅ Устанавливается таймер snooze
- ✅ Очередь уведомлений НЕ очищается (письма сохраняются)
- ✅ Проверки почты временно приостанавливаются

### При нажатии "Проверить сейчас" в трее:
- ✅ Вызывается команда `check_now()` (не `poll_once()`)
- ✅ Режим отложения сбрасывается (логируется)
- ✅ Если есть письма в очереди, окно показывается
- ✅ Выполняется проверка новых писем
- ✅ Новые письма добавляются в очередь

### После истечения времени отложения:
- ✅ Режим отложения автоматически сбрасывается
- ✅ Окно с уведомлениями показывается автоматически
- ✅ Проверки почты возобновляются по расписанию

## Логирование для отладки

При нажатии "Проверить сейчас" в трее теперь видно:
```
INFO tray click: check_now
INFO check_now: manual check requested
INFO check_now: snooze cleared            # если был snooze
INFO check_now: showing current notification from queue  # если есть письма
INFO check_now: calling poll_once
```

Если видите только `DEBUG gmail polling snoozed` без логов от `check_now`, значит обработчик трея не был исправлен.

## Тестирование

1. **Получите уведомление о письме**
2. **Нажмите "Отложить"** → окно скрывается
3. **Нажмите "Проверить сейчас" в трее**
   - Смотрите в консоль: должны быть логи от `check_now`
   - Окно должно появиться с тем же письмом
4. **Нажмите "Отложить" снова**
5. **Дождитесь окончания snooze** → окно появится автоматически

## Статус
✅ **Исправлено и собрано успешно**
- Все изменения применены
- Компиляция прошла без ошибок
- 4 предупреждения (не критичные, связаны с неиспользуемым кодом)

## Дата исправления
27.11.2025 (обновлено с исправлением обработчика трея)

