@echo off
echo ========================================
echo   Gmail Tray Notifier - Запуск
echo ========================================
echo.
echo Функционал: Отображение тела письма при двойном клике
echo.

echo [1/3] Проверка компонентов...
node scripts\check-implementation.js
if errorlevel 1 (
    echo.
    echo [!] Обнаружены проблемы. Исправьте их перед запуском.
    pause
    exit /b 1
)

echo.
echo [2/3] Компоненты проверены. Запуск приложения...
echo.
echo Приложение запускается в режиме разработки.
echo Окно браузера откроется автоматически.
echo.
echo Для остановки нажмите Ctrl+C
echo.
pause

echo [3/3] Запуск...
npm start

