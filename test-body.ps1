# Тестовый скрипт для отладки получения тела письма
Write-Host "Запуск приложения для тестирования получения тела письма..." -ForegroundColor Green
Write-Host "Следите за логами DEBUG для сообщений о структуре payload..." -ForegroundColor Yellow
Write-Host ""

# Устанавливаем переменную окружения для уровня логирования
$env:RUST_LOG = "debug"

# Запускаем приложение
& "C:\project\mail-agent\src-tauri\target\debug\gmail_tray_notifier.exe"

