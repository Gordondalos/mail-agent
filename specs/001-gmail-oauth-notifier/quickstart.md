# Quickstart: Mail Tray Notifier (Gmail OAuth)

1) Prepare OAuth client (Desktop App) with redirect:
   - http://localhost:42813/oauth2callback

2) Dev setup
   - Windows: `pwsh -File .\\scripts\\setup.ps1 -Auto -Dev`
   - Linux: `bash scripts/setup.sh --auto --dev`

3) Run in dev
   - `cd src-tauri && cargo tauri dev`

4) First run
   - Open Settings, connect account, set polling interval and sound file
   - Verify tray menu (Settings, Exit) and autostart toggle

5) Test notifications
   - Send yourself a new email; expect 600×150 alert with View/Skip
