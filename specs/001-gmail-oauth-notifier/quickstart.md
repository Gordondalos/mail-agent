# Quickstart: Mail Tray Notifier (Gmail OAuth)

1) Prepare OAuth client (Desktop App) with redirect:
   - http://localhost:42813/oauth2callback

2) Dev setup
   - Windows: `pwsh -File .\\scripts\\setup.ps1 -Auto`
   - Linux: `bash scripts/setup.sh --auto`

3) Run in dev (from repo root)
   - `npm run start` (запускает Angular dev‑server и `cargo tauri dev`)

4) First run
   - Open Settings, connect account, set polling interval and sound file
   - Verify tray menu (Settings, Exit) and autostart toggle

5) Test notifications
   - Send yourself a new email; expect 800×150 alert that stays until action
