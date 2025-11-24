# Инструкция по сборке

В этом руководстве описано, как собрать Gmail Tray Notifier для Windows и Linux.

## Предварительные требования

Убедитесь, что у вас установлены необходимые зависимости.
- **Windows**: Запустите `npm run install:windows`
- **Linux**: Запустите `npm run install:linux`

## Сборка для Windows

Чтобы собрать исполняемый файл для Windows (`.exe`), выполните следующую команду **на машине с Windows**:

```bash
npm run build:windows
```

Это выполнит следующие действия:
1. Соберет Angular frontend.
2. Соберет Tauri backend.
3. Поместит артефакты в папку `release/`.

**Артефакты:**
- `release/gmail-tray-notifier_x.x.x_x64-setup.exe` (Установщик)
- `release/gmail-tray-notifier_x.x.x_x64_en-US.msi` (MSI установщик)

## Сборка для Linux

Чтобы собрать AppImage (`.AppImage`) и Debian пакет (`.deb`) для Linux, выполните следующую команду **на машине с Linux**:

```bash
npm run build:linux
```

Это выполнит следующие действия:
1. Соберет Angular frontend.
2. Соберет Tauri backend.
3. Поместит артефакты в папку `release/`.

**Артефакты:**
- `release/gmail-tray-notifier_x.x.x_amd64.AppImage` (AppImage)
- `release/gmail-tray-notifier_x.x.x_amd64.deb` (Debian пакет)

## Кросс-компиляция

**Примечание:** Кросс-компиляция (сборка приложений Windows на Linux или наоборот) не поддерживается скриптами этого проекта из-за платформозависимых зависимостей. Пожалуйста, выполняйте сборку на целевой операционной системе.
