<!--
Sync Impact Report
- Version change: N/A → 1.0.0
- Modified principles:
  - PRINCIPLE_1_NAME → Security & Secrets Hygiene (NON-NEGOTIABLE)
  - PRINCIPLE_2_NAME → Code Style & Tooling Discipline
  - PRINCIPLE_3_NAME → Testing Discipline
  - PRINCIPLE_4_NAME → Build, Dev & Release Flow
  - PRINCIPLE_5_NAME → Observability & Simplicity
- Added sections:
  - Additional Constraints & Stack
  - Development Workflow & Reviews
- Removed sections: none
- Templates requiring updates:
  - ✅ .specify/templates/plan-template.md (remove outdated commands path)
  - ✅ .specify/templates/spec-template.md (aligned; no changes needed)
  - ✅ .specify/templates/tasks-template.md (aligned; no changes needed)
  - ✅ .specify/templates/agent-file-template.md (aligned; no changes needed)
  - ✅ .specify/templates/checklist-template.md (aligned; no changes needed)
  - ⚠ .specify/templates/commands/*.md (folder not present; no action)
- Follow-up TODOs:
  - TODO(RATIFICATION_DATE): maintainers to provide original adoption date
-->

# Mail Agent Constitution

## Core Principles

### 0. Русский язык коммуникаций и документации (NON-NEGOTIABLE)
Коммуникации и документация проекта ведутся на русском языке.

- MUST: Во всех интерактивных сообщениях и обсуждениях использовать русский язык.
- MUST: Все артефакты спецификаций и планирования (spec.md, plan.md, tasks.md,
  research.md, checklists) создаются и поддерживаются на русском языке.
- MUST: Документация в репозитории (README, руководства, файлы в specs/) — на
  русском. Если требуется двуязычие, русская версия обязательна.
- SHOULD: Комментарии в коде и строки интерфейса локализуются на русский, если
  нет внешних требований к английскому (например, названия API/ошибок).

Rationale: Единый язык команды снижает риски недопонимания и ускоряет ревью.

### I. Security & Secrets Hygiene (NON-NEGOTIABLE)
MUST protect user credentials and application secrets at all times.

- MUST NOT commit OAuth Client Secret, refresh/access tokens, or sensitive
  credentials to the repository.
- OAuth redirect URI is fixed: `http://localhost:42813/oauth2callback`. Any
  change requires explicit security review and documentation updates.
- Use temporary credentials for debugging only; MUST revoke them when finished.
- Secrets MUST be stored securely (e.g., OS keychain via `keyring` crate) and
  never logged.

Rationale: Minimizes risk of credential leakage and complies with Google OAuth
policies and user expectations.

### II. Code Style & Tooling Discipline
Ensure consistent, high-quality code across Rust backend and static frontend.

- Rust: edition 2021. Before commit, run from `src-tauri/`:
  - `cargo fmt`
  - `cargo clippy --all-targets -- -D warnings`
  - Use `cargo check` prior to opening a PR.
- Naming: descriptive types; functions in `snake_case`; constants in
  `SCREAMING_SNAKE_CASE`. Modules grouped by responsibility: config, OAuth,
  Gmail client, notifier.
- Frontend: 2-space indent; semicolons required; prefer `const`; vanilla DOM
  only—introducing frameworks requires prior approval.
- Build artifacts under `src-tauri/target/` are temporary and MUST NOT be
  committed.

Rationale: Enforces uniformity and catches issues early via linting.

### III. Testing Discipline
Testing is mandatory and scoped to where changes occur.

- Each Rust file ends with a `#[cfg(test)]` module; give tests meaningful names
  (e.g., `handles_refresh_token_loss`).
- Run `cd src-tauri && cargo test` locally. For OAuth/Gmail paths, emulate HTTP
  using utilities from `oauth2` instead of hitting live services.
- Manual smoke test before review: login flow, unread polling, notification
  buttons, tray commands.

Rationale: Prevents regressions in critical auth and notification flows.

### IV. Build, Dev & Release Flow
Ensure reproducible setup, fast iteration, and reliable packaging.

- One-shot setup/dev:
  - Windows: `pwsh -File .\scripts\setup.ps1 -Auto -Dev`
  - Linux: `bash scripts/setup.sh --auto --dev`
- Direct Tauri workflow: `cd src-tauri && cargo tauri dev` (watch logs via
  `tracing`).
- Release: `cd src-tauri && cargo tauri build`. Scripts copy installers to
  `release/` at repo root. Do not commit `src-tauri/target/` artifacts.

Rationale: Streamlines developer onboarding and release consistency.

### V. Observability & Simplicity
Prefer simple designs and sufficient telemetry for debugging.

- Use structured logging (`tracing`) and log key lifecycle events: auth,
  polling, queue operations, user actions.
- Keep implementations minimal (YAGNI). Avoid introducing dependencies that do
  not clearly pay for their complexity, especially frontend frameworks.
- Maintain clear module boundaries (config, OAuth, Gmail, notifier) for
  traceability and testability.

Rationale: Simpler code is easier to reason about and support.

## Additional Constraints & Stack

- Platform: Tauri desktop app with Rust backend (`src-tauri/`) and static
  frontend (`frontend/`).
- Language: Rust (edition 2021) for backend; vanilla JS/CSS for frontend.
- OAuth: Google OAuth2 with PKCE; local redirect at
  `http://localhost:42813/oauth2callback`.
- Secrets and tokens stored in OS keychain via `keyring` crate (or equivalent
  secure storage); never in source control.
- Build artifacts in `src-tauri/target/` are ephemeral; release installers are
  copied to `release/`.

## Development Workflow & Reviews

- Commits: short, present tense (often in Russian), e.g., «Обновляю обработку
  OAuth». Separate unrelated changes.
- PRs: describe changes; list manual checks performed; link tasks; attach UI
  screenshots/videos when applicable; request reviewers for Rust and frontend if
  both areas are affected.
- Pre-PR checks: `cargo check`, `cargo fmt`, `cargo clippy --all-targets -- -D
  warnings`, `cargo test`, complete manual smoke test.

## Governance

- Supremacy: This constitution governs engineering practices for this project
  and supersedes conflicting informal conventions.
- Amendments: Propose via PR with rationale, risk assessment, and migration
  plan (if any). Mark changes as MAJOR/MINOR/PATCH per versioning policy.
- Versioning Policy (for this constitution):
  - MAJOR: Backward-incompatible governance changes (e.g., removing or
    redefining a principle).
  - MINOR: New principle/section added or materially expanded guidance.
  - PATCH: Clarifications and non-semantic refinements.
- Compliance Reviews: All PRs must verify adherence to Language (Russian), Security, Style/Tooling, Testing, Build/Release, and Observability principles. Violations must include
  a justification and time-bounded remediation plan.
- Constitution Check Gate: Planning artifacts (e.g., `/speckit.plan`) must
  perform a Constitution Check covering: secrets handling, testing strategy,
  code style/lint strategy, build/dev flow alignment, and observability. Plans
  that fail the gate must be revised before implementation.

**Version**: 1.1.0 | **Ratified**: TODO(RATIFICATION_DATE): maintainers to provide original adoption date | **Last Amended**: 2025-10-24

