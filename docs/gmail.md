# Настройка доступа к Gmail (OAuth)

Короткое руководство по подключению Gmail API и получению токенов доступа.

1) Создать проект в Google Cloud Console
- Перейдите в https://console.cloud.google.com/
- Создайте новый проект или используйте существующий
- Включите Gmail API (APIs & Services → Library → Gmail API)

2) Создать учетные данные OAuth 2.0
- APIs & Services → Credentials → Create Credentials → OAuth client ID
- Тип: "Desktop app" или "Web application" в зависимости от схемы авторизации
- Сохраните `client_id` и `client_secret`

3) Настройка redirect URI
- Для desktop-приложений обычно используется `urn:ietf:wg:oauth:2.0:oob` или loopback адрес (http://127.0.0.1:xxxx)

4) Интеграция в проект
- Поместите `client_id`/`client_secret` в защищённый конфиг. В этом проекте конфиг и логика авторизации находятся в `src-tauri/` (файлы `oauth.rs`, `config.rs`).
- При первом запуске приложение должно открыть страницу авторизации и сохранить refresh token в конфиг или файл токенов.

5) Безопасность
- Не храните `client_secret` в публичных репозиториях.
- Рассмотрите использование зашифрованного хранилища или системных менеджеров секретов.

Примечание: более детальный пример запроса к Gmail API и логики обновления токенов можно разместить здесь позже вместе с выдержками из `src-tauri/oauth.rs`.
