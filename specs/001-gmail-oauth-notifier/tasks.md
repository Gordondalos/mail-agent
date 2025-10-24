---

description: "Task list for Mail Tray Notifier (Gmail OAuth)"
---

# Tasks: Mail Tray Notifier (Gmail OAuth)

**Input**: Design documents from `C:\\project\\mail-agent\\specs\\001-gmail-oauth-notifier\\`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Tests are OPTIONAL and not requested in the spec; no test tasks included.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- [P]: Can run in parallel (different files, no dependencies)
- [Story]: Which user story this task belongs to (US1, US2, US3)
- Include exact file paths in descriptions

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure (Angular + Tauri integration)

- [x] T001 Initialize Angular workspace in frontend/ (create Angular project files) at frontend/
- [x] T002 Add Angular Material and theme configuration at frontend/package.json and frontend/angular.json
- [x] T003 [P] Scaffold components directories (tray-window, settings-page, notification-overlay) at frontend/src/app/components/
- [x] T004 [P] Create IPC service scaffold for Tauri bridge at frontend/src/app/services/ipc.service.ts
- [x] T005 Configure Tauri frontend distDir and allowlist (shell/open) at src-tauri/tauri.conf.json
- [x] T006 Ensure Rust module files present (config.rs, gmail.rs, notifier.rs, oauth.rs, main.rs) at src-tauri/src/

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

- [x] T007 Implement settings model + load/save to file at src-tauri/src/config.rs
- [x] T008 Implement OAuth PKCE flow and secure token storage at src-tauri/src/oauth.rs
- [x] T009 Implement Gmail unread fetch and mark-read operations at src-tauri/src/gmail.rs
- [x] T010 Implement notifier queue and event emission to frontend at src-tauri/src/notifier.rs
- [x] T011 Expose Tauri commands (auth/connect, settings get/save, unread list, mark read, open link) at src-tauri/src/main.rs
- [x] T012 Update Angular build integration (serve/build paths) at frontend/angular.json

// Номера задач глобально последовательные; следующие задачи относятся к фундаментальным
- [x] T044 Implement polling scheduler loop driven by settings interval (start/stop, tick handler) at src-tauri/src/main.rs
- [x] T045 Reconfigure polling on settings change and account (connect/logout) state at src-tauri/src/main.rs

**Checkpoint**: Foundation ready — user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Connect Account and Configure (Priority: P1) 🎯 MVP

**Goal**: User connects account and configures polling interval, sound file, and autostart

**Independent Test**: New user completes connect + save settings; values persist after app restart

### Implementation for User Story 1

- [x] T013 [US1] Implement SettingsPageComponent form UI (fields: polling interval, sound file, autostart, connect button) at frontend/src/app/components/settings-page/settings-page.component.ts
- [x] T014 [P] [US1] Implement SettingsService (get/save) calling Tauri commands at frontend/src/app/services/settings.service.ts
- [x] T015 [US1] Wire OAuth connect button to Tauri `auth/connect` and handle completion at frontend/src/app/components/settings-page/settings-page.component.ts
- [ ] T016 [US1] Wire settings get/save commands in Tauri at src-tauri/src/main.rs

**Checkpoint**: User can connect account and manage settings; values persist

---

## Phase 4: User Story 2 - Receive Alerts for New Emails (Priority: P2)

**Goal**: Show 600×150 alert for new unread emails with View/Skip actions

**Independent Test**: Creating a new unread email shows alert with correct subject; View opens browser; Skip marks read

### Implementation for User Story 2

- [x] T020 [US2] Implement NotificationOverlayComponent UI (фиксированный размер 800×150, кнопки «Просмотреть»/«Пропустить») at frontend/src/app/components/notification-overlay/notification-overlay.component.ts
- [x] T021 [P] [US2] Subscribe to Tauri events (new email notifications) in IPC service at frontend/src/app/services/ipc.service.ts
- [x] T022 [US2] Реализовать действие «Просмотреть» (открытие браузера по умолчанию) at src-tauri/src/main.rs
- [x] T023 [US2] Реализовать действие «Пропустить» (пометить письмо прочитанным и закрыть оверлей) at src-tauri/src/gmail.rs

- [x] T024 [US2] Запретить автоскрытие: убрать таймеры; оверлей остаётся до действия at frontend/src/app/components/notification-overlay/notification-overlay.component.ts
- [x] T025 [US2] Обрабатывать очередь только после действия (Просмотреть/Пропустить) перед показом следующего at src-tauri/src/notifier.rs

**Checkpoint**: Alerts display and actions work end-to-end

---

## Phase 5: User Story 3 - Tray Menu and Autostart Settings (Priority: P3)

**Goal**: Control app via tray (Settings, Exit) and toggle autostart

**Independent Test**: Tray shows menu; autostart enabled by default and can be toggled off

### Implementation for User Story 3

- [x] T030 [US3] Add tray menu (Settings, Exit) and handlers at src-tauri/src/main.rs
- [ ] T031 [US3] Implement autostart get/set Tauri commands and platform wiring at src-tauri/src/main.rs
- [ ] T032 [P] [US3] Bind Settings UI toggle to autostart commands at frontend/src/app/components/settings-page/settings-page.component.ts
- [x] T033 [US3] Persist autostart preference in settings at src-tauri/src/config.rs
- [x] T034 [US3] Enable autostart by default on first run (if user has not opted out) at src-tauri/src/main.rs

**Checkpoint**: Tray menu operational; autostart preference behaves as expected

---

## Phase N: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [x] T040 Update quickstart.md with Angular setup steps at specs/001-gmail-oauth-notifier/quickstart.md
- [ ] T041 [P] Localize UI strings to Russian at frontend/src/app/
- [ ] T042 [P] Add structured logging for auth/poll/queue/actions at src-tauri/src/
- [ ] T043 Tune default polling interval and validation at src-tauri/src/config.rs

---

## Dependencies & Execution Order

### Phase Dependencies

- Setup (Phase 1): No dependencies — can start immediately
- Foundational (Phase 2): Depends on Setup completion — BLOCKS all user stories
- User Stories (Phase 3+): All depend on Foundational phase completion
  - P1 → P2 → P3 in sequence for validation; parallel allowed after Phase 2 if staffed
- Polish (Final): Depends on desired user stories being complete

### User Story Dependencies

- User Story 1 (P1): No story dependencies; requires Phase 2
- User Story 2 (P2): Depends on notifier and Gmail operations (Phase 2)
- User Story 3 (P3): Depends on settings (Phase 2) and main tray wiring (Phase 2)

### Parallel Opportunities

- T003, T004 can run in parallel (distinct frontend files)
- T014 parallel to T013
- T021 parallel to T020
- T032 parallel to T030/T031 once commands exist
- T041, T042 can run in parallel across separate trees

---

## Parallel Example: User Story 2

```bash
Task: "Subscribe to Tauri events in IPC service" (T021)
Task: "Implement NotificationOverlayComponent UI" (T020)
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL — blocks all stories)
3. Complete Phase 3: User Story 1
4. STOP and VALIDATE: Test User Story 1 independently
5. Deploy/demo if ready

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → Demo (MVP)
3. Add User Story 2 → Test independently → Demo
4. Add User Story 3 → Test independently → Demo
5. Each story adds value without breaking previous stories
