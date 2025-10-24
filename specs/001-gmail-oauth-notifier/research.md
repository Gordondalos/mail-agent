# Research: Mail Tray Notifier (Gmail OAuth)

## Summary of Decisions

- Decision: Provider authentication policy = OAuth-first; app-specific passwords allowed for non-OAuth providers; regular passwords not supported.
  - Rationale: Minimizes security risk and aligns with provider best practices while enabling future non-Google providers.
  - Alternatives considered:
    - OAuth-only everywhere — most secure but blocks some providers.
    - Allow regular passwords — broad coverage but higher risk and compliance burden.

- Decision: Multiple new emails presentation = queue one-by-one.
  - Rationale: Clear user experience; aligns with existing queue semantics in project; prevents alert flood.
  - Alternatives considered: show only latest; group summary view.

- Decision: Login/password field visibility = hidden unless provider requires passwords (or app-specific passwords).
  - Rationale: Avoids confusing OAuth-centric users; shows fields only when relevant to selected provider.
  - Alternatives considered: always show; hide behind Advanced.

- Decision: Frontend stack = Angular (latest) + Angular Material (latest).
  - Rationale: Explicit product requirement; consistent component library and theming.
  - Alternatives considered: vanilla DOM; other frameworks.

- Decision: Default polling interval = 60 seconds (configurable).
  - Rationale: Timely alerts without excessive API usage; user can adjust.
  - Alternatives considered: 30s (higher API load), 120s (slower alerts).

## Notes

- All UI strings to be localized in Russian before release.
- Follow security principle: no secrets in logs; tokens in OS keychain.
