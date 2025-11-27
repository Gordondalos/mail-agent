# Установка и запуск

В этом разделе собраны инструкции по подготовке окружения и запуску проекта на разных платформах.

Требования
- Node.js (LTS) и npm
- Rust toolchain (rustup, cargo)
- Tauri CLI (рекомендуется: `cargo install tauri-cli`)
- Windows: инструменты для сборки инсталляторов (NSIS/WIX) при необходимости

Установка зависимостей

Windows (PowerShell):

```powershell
# Установить зависимости Node.js
npm install

# Установить Rust (если ещё нет)
# https://rustup.rs/

# Установить tauri-cli
cargo install tauri-cli
```

Запуск в режиме разработки

```powershell
# Запустить фронтенд в dev-режиме и Tauri (см. package.json или scripts)
npm run start
```

Сборка для релиза

```powershell
# Сборка фронтенда
npm run build

# Сборка Tauri приложения
npm run build:windows
# или
npm run build:linux
```

Примечания
- Конкретные команды и скрипты находятся в `package.json` и в папке `scripts/`. Проверьте их перед выполнением.
- Если потребуется пакетирование под конкретную ОС (MSI/NSIS), посмотрите `release/` и `src-tauri` (wix, nsis конфиги).
