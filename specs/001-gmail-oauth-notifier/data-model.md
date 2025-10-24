# Data Model: Mail Tray Notifier (Gmail OAuth)

## Entities

- Account
  - Fields: id (provider-scoped), email, provider (enum), connected (bool)
  - Relationships: 1..* Credentials (historical), 1 Settings

- Credential
  - Fields: id, provider, created_at, scopes, secure_handle (keychain ref)
  - Notes: no raw secrets persisted in files/logs

- Settings
  - Fields: polling_interval_seconds, sound_file_path, autostart_enabled,
    provider_requires_password (derived), locale
  - Validation: polling_interval_seconds >= 15; sound_file_path must exist

- EmailItem
  - Fields: id, thread_id (if applicable), subject, link, received_at,
    unread (bool)

- Notification
  - Fields: id, email_item_id, shown_at, action (view/skip/timeout)
  - State: queued → visible → resolved

## State Transitions

- EmailItem: unread=true → unread=false on Skip or external read
- Notification: queued → visible when slot free; visible → resolved on action

## Derived Rules

- provider_requires_password is false for OAuth providers; true only for
  providers configured with app-specific passwords.
