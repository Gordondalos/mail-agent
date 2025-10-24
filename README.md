# Gmail Tray Notifier

Кроссплатформенное Tauri-приложение, которое висит в системном трее, проверяет непрочитанные письма Gmail и показывает кастомный полупрозрачный алерт 650×150 с кнопками «Перейти» и «Прочитано», а также воспроизводит звук уведомления.

## Быстрый старт

Ниже — краткий путь, как запустить и настроить приложение у себя локально.

1) Подготовьте OAuth2 в Google (один раз):
- Создайте OAuth Client ID типа Desktop App (см. раздел «Подготовка OAuth2» ниже). Для Desktop App не нужно вручную добавлять redirect URI — приложение само слушает `http://localhost:42813/oauth2callback`.

2) Установите зависимости для вашей ОС:
- Windows: запустите PowerShell-скрипт `scripts\setup.ps1 -Auto` (или поставьте всё вручную — см. раздел ниже).
- macOS: выполните команды из раздела «Установка зависимостей (macOS)».
- Linux: запустите `bash scripts/setup.sh --auto` (или поставьте всё вручную — см. раздел ниже).

3) Запуск в dev-режиме:
- В корне проекта выполните `npm run start` — поднимутся одновременно Angular dev‑сервер (порт 4200) и `cargo tauri dev`.

4) Первая настройка в приложении:
- В основном окне откройте «Настройки» и вставьте ваши `Client ID` и (опционально) `Client Secret`.
- При первом входе нажмите «Войти в Gmail» — откроется браузер с OAuth2.
- При желании укажите интервал опроса, путь к звуку, громкость, автозапуск и Gmail-запрос (по умолчанию: `is:unread category:primary`).

5) Использование:
- При новых письмах появится полупрозрачный алерт 800×150 (не автоскрывается) с кнопками:
  - «Перейти» — открыть письмо в браузере;
  - «Прочитано» — снять метку `UNREAD` с письма.

Если нужны installers/дистрибутив — соберите релиз командой `npm run build` (см. раздел «Сборка и запуск»).

## Возможности

- Авторизация через Google OAuth2 с использованием PKCE и локального редиректа `http://localhost:42813/oauth2callback`.
- Токены доступа хранятся в системном keychain (библиотека [`keyring`](https://crates.io/crates/keyring)).
- Очередь уведомлений: в интерфейсе всегда отображается только одно письмо, остальные ждут своей очереди.
- Кнопка «Перейти» открывает письмо в браузере, «Прочитано» снимает метку `UNREAD` с письма.
- Настраиваемый интервал опроса, путь до мелодии, громкость, автозапуск и кастомный запрос Gmail (например `is:unread category:primary`).
- Полупрозрачный алерт 800×150 px; не автоскрывается — ожидает действие пользователя.
- Звуковое уведомление (поддерживаются локальные файлы `.mp3`/`.wav`).

## Структура проекта

```
frontend/            # Angular фронтенд (основное окно и алерт)
src-tauri/           # Rust-бэкенд Tauri
  ├── src/
  │   ├── config.rs  # Работа с настройками
  │   ├── gmail.rs   # Клиент Gmail API и формирование уведомлений
  │   ├── notifier.rs# Очередь уведомлений и события для фронтенда
  │   ├── oauth.rs   # OAuth2 + хранение токенов в keychain
  │   └── main.rs    # Сборка приложения, системный трей, команды
  └── tauri.conf.json
spec-kit/            # Подмодуль Spec Kit для спецификаций и планирования
```

## Spec Kit и спецификации

В корне подключён git-подмодуль `spec-kit/`, содержащий инструменты Spec Kit и документацию по спецификационному процессу.

- Инициализация или обновление: `git submodule update --init --recursive`.
- Установка CLI локально (если нужен `specify`): `uv tool install specify-cli --from git+https://github.com/github/spec-kit.git`. Если `uv` не установлен, используйте `pip install --upgrade "specify-cli @ git+https://github.com/github/spec-kit.git"`.
- Дополнительные материалы и готовые шаблоны находятся в `spec-kit/templates/` и `spec-kit/docs/`.

## Подготовка OAuth2

Нужно один раз создать OAuth‑клиент в Google Cloud Console. Далее этим клиентом могут пользоваться все пользователи вашего приложения.

Шаги:
- Настройте OAuth consent screen (External). Если приложение в режиме Testing, добавьте e‑mail пользователей в Test users.
- Создайте OAuth Client ID типа «Desktop App». Для Desktop App Google не требует настраивать redirect URI — локальный редирект `http://localhost:42813/oauth2callback` допустим по умолчанию.
- Скопируйте `Client ID` и, при желании, `Client Secret`.

Как использовать в приложении:
- Откройте окно настроек и вставьте `Client ID`. `Client Secret` опционален (используем PKCE; секрет не обязателен для Desktop App).
- Нажмите «Войти в Gmail» и завершите OAuth в браузере. Токены сохраняются только локально в системном хранилище (keychain), секреты — в `settings.json` (см. раздел «Настройки»).

Распространение среди друзей/коллег:
- Достаточно одного OAuth Client ID на всё приложение. Пользователям не нужно создавать свои ключи — они просто авторизуются через ваше приложение.
- Если у вас включён режим Testing в OAuth consent screen, добавьте их e‑mail в Test users. Иначе они увидят предупреждение «Unverified app» или не смогут войти.

## Настройки

Настройки сохраняются в файле `settings.json` в платформенной директории конфигурации, определяемой через `directories::ProjectDirs` с идентификатором `org/kreditpro/GmailTrayNotifier`.

Примечание: точный путь зависит от ОС (например, `%APPDATA%\org\kreditpro\GmailTrayNotifier\settings.json` на Windows, `~/Library/Application Support/org.kreditpro.GmailTrayNotifier/settings.json` на macOS, `~/.config/org.kreditpro/GmailTrayNotifier/settings.json` на Linux).

Параметры можно менять через UI или вручную (JSON):

```json
{
  "poll_interval_secs": 30,
  "sound_enabled": true,
  "sound_path": "/path/to/notify.wav",
  "auto_launch": true,
  "gmail_query": "is:unread category:primary",
  "oauth_client_id": "...",
  "oauth_client_secret": "...",
  "playback_volume": 0.7
}
```

FAQ по OAuth и запуску:
- Нужно ли получать Client ID и Secret? Да, нужен один OAuth Client ID типа Desktop App. Secret не обязателен (PKCE), но можно хранить локально.
- Куда класть ключи? Введите в настройках приложения. Они сохраняются в `settings.json`; токены доступа — в системном keychain.
- Сколько раз получать ключи? Один раз на приложение. Все пользователи могут использовать один и тот же Client ID.
- Должны ли друзья делать то же самое? Нет. Им достаточно авторизоваться в вашем приложении. Если консент‑экран в режиме Testing — добавьте их e‑mail в Test users.
- Где это работает? На Windows/macOS/Linux. Редирект идёт на `http://localhost:42813/oauth2callback` в системном браузере; убедитесь, что фаервол не блокирует localhost.

## Установка зависимостей (Windows)

Есть два пути: автоматизированный (рекомендуется) и ручной.

- Автоматизированный: используйте скрипт PowerShell `scripts\\setup.ps1`. Он проверит наличие всего, что нужно, и при флаге `-Auto` попытается установить недостающие глобальные зависимости через winget/choco/scoop, включая Rust, Node.js, WebView2 и Tauri CLI.
- Ручной: установите каждую зависимость командами ниже.

1) Автоматизированная установка

1. Откройте PowerShell от имени пользователя (или администратора — так выше шанс автоустановки глобальных пакетов).
2. В корне репозитория выполните:

```powershell
pwsh -File .\scripts\setup.ps1 -Auto
```

Опции скрипта:
- `-Auto` — попытаться автоматически установить недостающие компоненты через winget/choco/scoop (если доступны).
- `-Dev` — после проверки запустить `cargo tauri dev`.
- `-Build` — после проверки запустить `cargo tauri build`.

Примеры:

```powershell
# Автоустановка всего и запуск dev
pwsh -File .\scripts\setup.ps1 -Auto -Dev

# Только проверка без установки
pwsh -File .\scripts\setup.ps1
```

2) Ручная установка (однострочники)

Выполните по возможности один из наборов команд (любой пакетный менеджер):

- Через winget:

```powershell
winget install -e --id OpenJS.NodeJS.LTS ; `
winget install -e --id Rustlang.Rustup ; `
winget install -e --id Microsoft.EdgeWebView2Runtime ; `
winget install -e --id Microsoft.VisualStudio.2022.BuildTools
```

После установки rustup выполните (однократно) настройку MSVC toolchain:

```powershell
rustup default stable-x86_64-pc-windows-msvc
rustup component add rust-src
```

- Через Chocolatey:

```powershell
choco install -y nodejs-lts rustup.install microsoft-edge-webview2-runtime visualstudio2022buildtools
```

- Через Scoop (если установлен):

```powershell
scoop install nodejs-lts
```

Установка Tauri CLI (любой вариант):

```powershell
# через cargo (предпочтительно)
cargo install tauri-cli
# либо через npm
yarn global add @tauri-apps/cli  # или: npm i -g @tauri-apps/cli
```

Требуемые компоненты (Windows):
- Node.js (LTS) и npm
- Rust (rustup/cargo) с MSVC toolchain (stable-x86_64-pc-windows-msvc)
- Visual Studio Build Tools (Desktop development with C++)
- WebView2 Runtime
- Tauri CLI (`cargo install tauri-cli` или `npm i -g @tauri-apps/cli`)

## Установка зависимостей (macOS)

Для macOS достаточно установить Xcode Command Line Tools, Node.js, Rust и Tauri CLI. Рекомендуется использовать Homebrew.

1) Быстрый старт через Homebrew

```bash
# Установите Homebrew, если его нет: https://brew.sh/
xcode-select --install               # Xcode Command Line Tools (однократно)
brew install node                    # Node.js + npm
# Rustup
curl https://sh.rustup.rs -sSf | sh -s -- -y
source "$HOME/.cargo/env"
# Tauri CLI (через cargo или npm)
cargo install tauri-cli  # или: npm i -g @tauri-apps/cli
```

2) Примечания (macOS)
- Если при сборке падают нативные зависимости, обновите CLT: `xcode-select --install`.
- На macOS используется WebView (входит в систему), дополнительных WebKitGTK пакетов не требуется.
- После установки rustup выполните `source "$HOME/.cargo/env"` в текущей сессии или перезапустите терминал.

## Установка зависимостей (Linux)

Для Linux добавлен скрипт `scripts/setup.sh`, который умеет автоматически ставить всё необходимое на дистрибутивах Debian/Ubuntu (apt), Fedora (dnf) и Arch (pacman).

1) Автоматизированная установка

```bash
bash scripts/setup.sh --auto
# Автоустановка и запуск dev
bash scripts/setup.sh --auto --dev
```

Что установится:
- Системные библиотеки для Tauri (WebKitGTK, GTK3, appindicator, OpenSSL, pkg-config, инструменты сборки)
- Node.js и npm
- Rust (rustup + cargo)
- Tauri CLI (через cargo или npm — в зависимости от доступности)

2) Ручная установка по дистрибутивам

- Debian/Ubuntu:

```bash
sudo apt update
# Системные зависимости
if apt-cache show libwebkit2gtk-4.1-dev >/dev/null 2>&1; then \
  sudo apt install -y build-essential curl wget libssl-dev pkg-config libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev; \
else \
  sudo apt install -y build-essential curl wget libssl-dev pkg-config libgtk-3-dev libwebkit2gtk-4.0-dev libayatana-appindicator3-dev librsvg2-dev; \
fi
# Node.js + npm
sudo apt install -y nodejs npm
# Rustup + cargo
curl https://sh.rustup.rs -sSf | sh -s -- -y
source "$HOME/.cargo/env"
# Tauri CLI
cargo install tauri-cli  # или: npm i -g @tauri-apps/cli
```

- Fedora:

```bash
# Системные зависимости
if dnf info webkit2gtk4.1-devel >/dev/null 2>&1; then \
  sudo dnf install -y @"Development Tools" curl wget openssl-devel pkgconf-pkg-config gtk3-devel webkit2gtk4.1-devel libappindicator-gtk3 librsvg2-devel; \
else \
  sudo dnf install -y @"Development Tools" curl wget openssl-devel pkgconf-pkg-config gtk3-devel webkit2gtk3-devel libappindicator-gtk3 librsvg2-devel; \
fi
# Node.js + npm
sudo dnf install -y nodejs npm
# Rustup + cargo
curl https://sh.rustup.rs -sSf | sh -s -- -y
source "$HOME/.cargo/env"
# Tauri CLI
cargo install tauri-cli  # или: npm i -g @tauri-apps/cli
```

- Arch Linux:

```bash
# Системные зависимости
sudo pacman -Sy --needed --noconfirm base-devel curl wget pkgconf openssl gtk3 webkit2gtk libappindicator-gtk3 librsvg
# Node.js + npm
sudo pacman -Sy --needed --noconfirm nodejs npm
# Rustup + cargo (если не установлен)
sudo pacman -Sy --needed --noconfirm rustup
rustup default stable
# Tauri CLI
cargo install tauri-cli  # или: npm i -g @tauri-apps/cli
```

Примечания (Linux):
- На некоторых системах доступна только WebKitGTK 4.0 — это нормально; при наличии 4.1 лучше использовать её.
- После установки rustup обязательно выполните `source "$HOME/.cargo/env"` в текущей сессии или перезапустите терминал.
- Если используете Wayland и возникает проблема с индикатором трея, убедитесь, что запущен совместимый апплет трея (например, в KDE/Plasma, GNOME с расширениями и т.д.).

## Сборка и запуск

Коротко:
- Windows (одной командой): `pwsh -File .\scripts\setup.ps1 -Auto -Dev` для запуска в dev-режиме, или `-Auto -Build` для сборки установщика.
- Linux (одной командой): `bash scripts/setup.sh --auto --dev` для dev, или `--auto --build` для сборки.
- macOS: установите зависимости согласно документации Tauri, затем из каталога `src-tauri` выполните `cargo tauri dev` или `cargo tauri build`.

После сборки релиза скрипты автоматически копируют артефакты в папку `release` в корне репозитория, чтобы было удобно найти всё, что нужно для распространения.

Пошагово вручную:
1. Установите зависимости Tauri согласно [официальной документации](https://tauri.app/v1/guides/getting-started/prerequisites/) или используйте скрипт `scripts/setup.ps1` на Windows / `scripts/setup.sh` на Linux.
2. Установите `@tauri-apps/cli` (глобально через cargo или npm).
3. В корне проекта запустите:

```bash
cd src-tauri
cargo tauri dev
```

Или для сборки релиза:

```bash
cargo tauri build
```

Фронтенд — статический, дополнительных сборок не требуется (используется `frontend/` как dev/dist каталог).

### Где найти бинарники/установщики после сборки
После `cargo tauri build`:
- Скопированные артефакты лежат в папке `release/` в корне проекта (скрипты сохраняют туда EXE/MSI/NSIS на Windows, AppImage/DEB/RPM на Linux, DMG/APP на macOS). 
- Также оригинальные файлы находятся в стандартных путях Tauri:
  - Windows:
    - Портативный бинарник: `src-tauri\target\release\gmail_tray_notifier.exe`
    - Установщик MSI/NSIS: `src-tauri\target\release\bundle\` (подпапки `msi` или `nsis`)
  - Linux:
    - Портативный бинарник: `src-tauri/target/release/gmail_tray_notifier`
    - Пакеты: `src-tauri/target/release/bundle/` (подпапки `deb`, `appimage`, `rpm` и т.п. — зависит от дистрибутива)
  - macOS:
    - Приложение: `src-tauri/target/release/bundle/macos/*.app`
    - DMG: `src-tauri/target/release/bundle/dmg/*.dmg`

> Примечание: точные имена файлов могут отличаться в зависимости от настроек `package.productName` и системных таргетов.

## Горячие клавиши и меню трея

- **Проверить сейчас** — немедленный опрос Gmail.
- **Войти в Gmail** — запуск OAuth2 в системном браузере.
- **Выйти** — удаляет токены и останавливает очередь уведомлений.
- **Выход из приложения** — завершает процесс.

## Диагностика

- Логи пишутся через `tracing` и доступны в стандартном выводе.
- Очередь уведомлений очищается после логаута.
- При ошибках сети приложение повторит запрос на следующем интервале.

## Проверка (smoke test)

1) Подготовьте OAuth: в Google Cloud Console создан Desktop OAuth Client с redirect `http://localhost:42813/oauth2callback`.
2) Dev‑запуск: в корне выполните `npm run start` и дождитесь главного окна.
3) Настройки: нажмите «Войти в Gmail», завершите OAuth; при необходимости отредактируйте интервал, звук, автозапуск — сохраните.
4) Алерт: отправьте себе письмо — ожидайте оверлей 800×150; проверьте действия «Перейти», «Прочитано», «Скрыть».
5) Трей: проверьте «Проверить сейчас», «Войти в Gmail», «Выйти», «Выйти из приложения».




/speckit.constitution — задать ваши принципы/ограничения проекта.

/speckit.specify — описать «что строим» (требования/юзер-стори).

/speckit.plan — зафиксировать стек и архитектурные решения.

/speckit.tasks — сгенерировать детализированный список задач.

/speckit.implement — исполнить задачи по плану.


/speckit.clarify — задаёт структурированные вопросы по неясностям до планирования (снижает риски «домыслов» в спеках). Пример:
/speckit.clarify Наш агент для почты: как обрабатываем OAuth, какие лимиты Gmail, нужен ли офлайн-кэш?
YouTube

/speckit.analyze — проверяет согласованность spec/plan/tasks, подсвечивает противоречия и пробелы перед реализацией. Пример:
/speckit.analyze Проверь согласованность оповещений в трее и фонового опроса Gmail
den.dev

/speckit.checklist — генерирует «чек-листы качества» (UX, безопасность, доступность, локализация и т.п.), чтобы не упустить нефункциональные требования. Пример:
/speckit.checklist Сформируй чек-лист: безопасность токенов, автозапуск, автообновление, i18n
