# Dreamland v1 implementation tasks

Derived from [`plan.md`](plan.md), ordered by dependency.

- [x] Slice 1: Expand site-neutral core models and capability traits.
  - Acceptance: API types cover replayable queries, popular windows, typed
    media selection, site errors, and common post/detail parent models.
  - Verify: core compile/tests and format check.
  - Files: `crates/dreamland-core/`, `docs/API-V1.md` only if implementation
    evidence requires a contract correction.

- [x] Slice 1: Implement Yande JSON normalization and fixture matrix.
  - Acceptance: array/v2 posts, tag metadata, popular windows, missing fields,
    malformed payloads, negative terms, and safe filtering are covered.
  - Verify: `cargo test -p dreamland-site-yandere`.
  - Files: `crates/dreamland-site-yandere/`.

- [ ] Slice 2: Add runtime-owned query sessions and typed site registry.
  - Acceptance: cancellation, stale suppression, continuation sealing,
    capability gating, suggestions, and replayable saved-query input work.
  - Verify: runtime contract tests and Tauri serialization tests.
  - Files: `crates/dreamland-runtime/`, `crates/dreamland-sites/`,
    `src-tauri/`.
  - Checkpoint: typed Yande queries, suggestions, capability gates,
    cancellation/stale-result primitives, and network policy are present;
    SQLite-backed saved-query/history persistence remains.

- [ ] Slice 3: Add SQLite local state and durable download queue.
  - Acceptance: enqueue/retry/cancel/restart, cache staging, atomic rename,
    `ExistingTarget`, metadata snapshots, and history work.
  - Verify: schema, queue, path-security, and local HTTP integration tests.
  - Files: `crates/dreamland-runtime/`.
  - Checkpoint: queue/history schema, restart recovery, async worker,
    cancellation/retry, cache staging, canonical paths, existing-target
    protection, typed commands, and the Downloads panel are present; saved
    query/history tables, `open_download`, full progress events, and cache
    reconciliation remain.

- [ ] Slice 4: Replace legacy IPC with v1 commands and build exploration UI.
  - Acceptance: US-Y-01..US-Y-08 pass with active-site data and no raw site
    inputs crossing the command boundary.
  - Verify: `bun run typecheck`, `bun run build`, UI workflow tests.
  - Files: `src-tauri/`, `src/lib/ipc.ts`, `src/App.tsx`, `src/styles.css`.
  - Checkpoint: latest/popular/search UI, tag suggestions, detail/settings
    inspectors, responsive shell, and accessibility fallbacks are present;
    queue/history/auth/favorites/pools and runtime UI evidence remain.

- [ ] Slice 5: Implement config lifecycle, browser auth, and favorite mutation.
  - Acceptance: US-Y-09, G-05, and G-06 are covered; no secret leakage.
  - Verify: mock tests, serialization tests, clean-profile smoke, authorized
    reversible live check.
  - Files: `crates/dreamland-runtime/`, `crates/dreamland-site-yandere/`,
    `src-tauri/`, `src/lib/ipc.ts`.

- [ ] Slice 6: Implement pools and freeze favorite-list read semantics.
  - Acceptance: US-Y-10/11, G-07, and G-09 are either proven or explicitly
    blocked with redacted receipts.
  - Verify: fixtures plus authorized live checks.
  - Files: site/runtime/Tauri/UI files as required by the frozen contract.

- [ ] Slice 7: Run release gate and prepare implementation PR.
  - Acceptance: no stale definitions, all local gates green, external gaps
    accurately reported.
  - Verify: `make check` and the complete Yande gate.
  - Files: only evidence/docs and required implementation corrections.
