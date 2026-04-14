# Gmail Tray Notifier

Кроссплатформенное Tauri-приложение, которое висит в системном трее, проверяет непрочитанные письма Gmail и показывает кастомный полупрозрачный алерт 650×150 с кнопками «Перейти» и «Прочитано», а также воспроизводит звук уведомления.

## Быстрый старт

Ниже — краткий путь, как запустить и настроить приложение у себя локально.

1) Подготовьте OAuth2 в Google (один раз):
- Создайте OAuth Client ID типа Desktop App (см. раздел «Подготовка OAuth2» ниже). Для Desktop App не нужно вручную добавлять redirect URI — приложение само слушает `http://localhost:42813/oauth2callback`.

2) Установите системные зависимости для вашей ОС:
- Windows: `npm run install:windows`
- macOS: `npm run install:macos`
- Linux: `npm run install:linux`

3) Установите JavaScript-зависимости проекта:

```bash
npm install
npm --prefix frontend install
```

4) Запуск в dev-режиме:
- В корне проекта выполните `npm run start` (алиас `npm run dev`) — поднимутся Angular dev‑сервер (порт 4200) и `cargo tauri dev`.

5) Первая настройка в приложении:
- В основном окне откройте «Настройки» и вставьте ваши `Client ID` и (опционально) `Client Secret`.
- При первом входе нажмите «Войти в Gmail» — откроется браузер с OAuth2.
- При желании укажите интервал опроса, путь к звуку, громкость, автозапуск и Gmail-запрос (по умолчанию: `is:unread category:primary`).

6) Использование:
- При новых письмах появится полупрозрачный алерт 800×150 (не автоскрывается) с кнопками:
  - «Перейти» — открыть письмо в браузере;
  - «Прочитано» — снять метку `UNREAD` с письма.

Если нужны installers/дистрибутив — используйте `npm run release` (алиас: `npm run build`).

## Возможности

- Авторизация через Google OAuth2 с использованием PKCE и локального редиректа `http://localhost:42813/oauth2callback`.
- Токены доступа хранятся в системном keychain (библиотека [`keyring`](https://crates.io/crates/keyring)).
- Очередь уведомлений: в интерфейсе всегда отображается только одно письмо, остальные ждут своей очереди.
- **Развёрнутый просмотр письма**: двойной клик по окну уведомления разворачивает его и показывает полное содержимое письма (HTML или текст).
- Кнопка «Перейти» открывает письмо в браузере, «Прочитано» снимает метку `UNREAD` с письма.
- Настраиваемый интервал опроса, путь до мелодии, громкость, автозапуск и кастомный запрос Gmail (например `is:unread category:primary`).
- Настраиваемые размеры развёрнутого окна (по умолчанию 800×600 px).
- Полупрозрачный алерт 650×150 px; не автоскрывается — ожидает действие пользователя.
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
- Можно импортировать готовый JSON: нажмите «Импортировать client_secret.json» в настройках и выберите файл, поля подставятся автоматически.
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
- Если при сборке падают нативные зависимости, об��овите CLT: `xcode-select --install`.
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

### Быстрый сценарий через npm run

```bash
npm run install:windows   # Windows
npm run install:linux     # Linux
npm run install:macos     # macOS
npm install
npm --prefix frontend install
npm run start
```

Для релизной пересборки после правок:

```bash
npm run release
```

### Что значит "установка" в этом проекте

Под "установкой" здесь имеются в виду **два отдельных шага**:

1. Системные зависимости (Rust, WebView2/GTK, Tauri prerequisites):
- Windows: `npm run install:windows`
- Linux: `npm run install:linux`
- macOS: `npm run install:macos` (или ручные команды из раздела выше)

2. npm-зависимости репозитория:

```bash
npm install
npm --prefix frontend install
```

Без обоих шагов сборка может не стартовать.

### Где build, где release

- `npm run dev` / `npm run start` — режим разработки (Angular + `cargo tauri dev`)
- `npm run build:frontend` — только production-сборка фронтенда
- `npm run release` — полная релизная сборка (frontend + Tauri + копирование артефактов в `release/`)
- `npm run build` — алиас на `npm run release` для совместимости
- `npm run build:windows` / `npm run build:linux` — платформенные обертки с проверкой ОС
- `npm run rebuild` — только пересборка Rust-части без упаковки installer

### Таблица npm-скриптов (что делает каждый)

| Скрипт | Что делает | Когда запускать | ОС / ограничения | Результат |
|---|---|---|---|---|
| `npm run install:windows` | Запускает `scripts/setup.ps1 -Auto`, проверяет/ставит системные зависимости для Tauri | Первый запуск на Windows или при проблемах с окружением | Только Windows | Подготовленное окружение (Rust, Node, WebView2, Tauri CLI и т.д.) |
| `npm run install:linux` | Запускает `scripts/setup.sh --auto`, ставит зависимости Tauri и инструменты | Первый запуск на Linux | Linux | Подготовленное окружение для dev/build |
| `npm run install:macos` | Ставит базовые зависимости через brew/rustup + Tauri CLI | Первый запуск на macOS | macOS | Подготовленное окружение для dev/build |
| `npm run dev` | Стартует `scripts/dev.js` (Angular dev server + `cargo tauri dev`) | Ежедневная разработка | Кроссплатформенно | Приложение в dev-режиме |
| `npm run start` | Алиас на `npm run dev` | То же, что `dev` | Кроссплатформенно | То же, что `dev` |
| `npm run build:frontend` | Делает только production build фронтенда (`frontend`) | Если нужно проверить только веб-часть | Кроссплатформенно | Production-ассеты фронтенда |
| `npm run build:release` | Полный pipeline релиза через `scripts/build.js` | Основная команда релизной пересборки | Кроссплатформенно | Артефакты Tauri + копия в `release/` |
| `npm run release` | Алиас на `npm run build:release` | Рекомендуемая команда релиза | Кроссплатформенно | То же, что `build:release` |
| `npm run build` | Алиас на `npm run build:release` (обратная совместимость) | Если привыкли к `build` | Кроссплатформенно | То же, что `release` |
| `npm run build:windows` | Проверяет, что ОС Windows, затем запускает `build:release` | Когда хотите явно «только Windows сборка» | Только Windows | Релизная сборка Windows |
| `npm run build:linux` | Проверяет, что ОС Linux, затем запускает `build:release` | Когда хотите явно «только Linux сборка» | Linux (не Windows) | Релизная сборка Linux |
| `npm run rebuild:rust` | `cargo build` для `src-tauri/Cargo.toml` | Быстро пересобрать Rust ядро без упаковки | Кроссплатформенно | Новый debug build Rust |
| `npm run rebuild` | Алиас на `npm run rebuild:rust` | Короткая команда для Rust пересборки | Кроссплатформенно | То же, что `rebuild:rust` |
| `npm run debug` | Запускает отладочный скрипт `scripts/debug.js` | Для диагностики | Зависит от скрипта | Отладочная информация |
| `npm run run:debug:windows` | Запускает debug exe из `src-tauri/target/debug` | Быстрый запуск уже собранного debug бинарника | Только Windows | Запущен `gmail_tray_notifier.exe` |
| `npm run run:dev` | Алиас на `run:debug:windows` | То же, что выше | Только Windows | То же, что `run:debug:windows` |

### Частый вопрос: build vs build:windows vs release vs rebuild

- `build` и `release` — в этом проекте это одно и то же (оба ведут в полный релизный pipeline).
- `build:windows` — тот же релизный pipeline, но с проверкой, что вы действительно на Windows.
- `rebuild` — **не релиз**: только `cargo build` (пересборка Rust), без упаковки установщика и без копирования в `release/`.
- `install:windows` — это не сборка приложения, а подготовка окружения (установка зависимостей для сборки/запуска).

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
