# Dreamland v1 implementation plan

Status: API/runtime foundation complete; 0.1.0 release hardening in progress,
2026-08-12. This plan starts from the merged API/user-story/architecture
design and implements one vertical slice at a time. The release gate in
`docs/YANDE-RELEASE-GATE.md` remains the definition of done.

## Objective

Ship the first useful Yande desktop workflow: browse safe posts, search by
tags with suggestions, inspect normalized detail, enqueue/download selected
media, inspect download history, configure the runtime, authenticate through a
browser session, manage remote favorites, and browse/request Yande pool ZIPs.

Pixiv and Twitter remain descriptor-only skeletons until a real adapter slice
has endpoint/auth fixtures and its own acceptance evidence.

## Design constraints

- React receives application DTOs only; site JSON, cookies, URLs, and paths
  remain behind Rust.
- `dreamland-sites` is the composition root; Tauri does not depend directly on
  individual site crates.
- SQLite owns saved queries, query history, cache metadata, and queue/history;
  remote favorites and pools remain site-owned.
- Queue writes are asynchronous, staged through local cache, and committed by
  atomic rename. Existing targets end as `ExistingTarget` without overwrite,
  deduplication, or arbitrary-file scanning.
- Y-07/G-07 stays blocked until authorized favorite-list read semantics are
  verified. No guessed endpoint may be implemented to make the gate green.
- The meaning of “one month space” is not part of v1 until explicitly defined.

## Ordered slices

### Slice 1 — site-neutral contract and Yande wire fixtures — complete

Implement the common parent models, typed site capabilities, operation/error
types, replayable query input, fixed popular-window input, media selection, and
Yande JSON DTO mappers. Keep raw Yande DTOs private to the site crate.

Acceptance:

- legacy array and v2 `/post.json` envelopes normalize into common posts;
- tags preserve exact names and category metadata where available;
- `PostRef` is `(site_id, post_id)` and no raw site JSON crosses the boundary;
- tag encoding preserves spaces and negative terms;
- popular day/week/month maps to explicit period + anchor date and fixed-window
  pagination;
- fixture tests cover malformed payloads, missing metadata, safe filtering,
  and rate-limit/error mapping.

Verify: `cargo test -p dreamland-core -p dreamland-site-yandere`,
`cargo check --workspace`, `bun run typecheck`, `bun run build`, and
`cargo fmt --all -- --check` passed locally.

### Slice 2 — runtime query sessions and site registry — in progress

Move query execution behind runtime-owned operation/session handles. Expand
`dreamland-sites` from descriptor gating into the registry/composition surface
used by runtime commands. Implement cancellation, stale-result suppression,
capability checks, tag suggestions, and query history without persistence
leaking site cursors.

Acceptance:

- Yande browse/search/suggestion commands use typed requests;
- continuation state is internal and bound to site/query/auth/expiry;
- saved-query persistence accepts only `ReplayableQuery`;
- unsupported Pixiv/Twitter skeleton capabilities fail explicitly;
- empty, cancelled, stale, malformed, rate-limited, and auth-required states
  map to stable runtime errors.

Verify: runtime unit/contract tests plus Tauri serialization tests.

Current checkpoint: typed Yande commands, tag suggestions, capability gating,
network policy, cancellation/stale-result session primitives, and SQLite-backed
saved-query/history persistence are present. Full continuation sealing and the
remaining API trait/registry review are still open.

### Slice 3 — SQLite local state and download queue — in progress

Add the runtime-owned SQLite store and current schema for saved queries, query
history, site cache, download queue, and download history. Replace the current
synchronous path download with the durable queue worker and local-cache
staging model.

Acceptance:

- enqueue returns immediately and survives restart;
- `BestAvailable` is the default; explicit variant overrides configuration;
- media/archive URLs are resolved by the site adapter and scheme-validated;
- canonical paths use site/post identity, never tags or posting-account names;
- existing targets abort before write and before rename;
- cancellation, retry, crash recovery, progress, and terminal history are
  observable;
- `open_download` opens only a runtime-owned completed record.

Verify: SQLite schema tests, queue state-machine tests, path/security tests,
and an integration test using a local HTTP fixture server.

Current checkpoint: SQLite current schema, saved-query/history tables, durable
queue/history rows, restart recovery, asynchronous worker, cancellation, retry,
cache staging, canonical site/post paths, existing-target protection, network
policy, runtime-owned file opening, typed Tauri/IPC commands, and the Downloads
panel are present. Full byte-progress events, cache reconciliation, and a
local HTTP integration fixture remain open.

### Slice 4 — Tauri command surface and default exploration UI — in progress

Replace legacy image loading wiring with the approved command DTOs and typed
IPC wrappers. Build the default Yande feed, compact site selector
(showing only active sites), tag search/autocomplete, popular tabs/date anchor,
post detail, and download progress/history states.

Acceptance:

- US-Y-01 through US-Y-08 pass locally;
- React cannot submit site URLs, paths, cookies, or raw DTOs;
- fixed popular windows do not render false next-page controls;
- detail view exposes complete tags, posting account, rating, dimensions,
  checksum, and variants;
- loading/empty/error/unsupported/existing-target states are visible.

Verify: `bun run typecheck`, `bun run build`, and UI workflow tests against
Tauri command fixtures.

Current checkpoint: latest/popular/tag-search flows, tag suggestions, a
selected-post inspector, settings inspector, responsive shell, durable
downloads/history, auth, favorites, pools, and visual fallback/accessibility
CSS are present. Interactive workflow evidence remains open.

### Slice 5 — configuration, auth, and remote favorite mutation

Implement the site config lifecycle, secret/session boundary, browser-session
challenge flow, Yande cookie detection, and authenticated favorite add/remove.
Keep favorite-list read capability absent until authorized semantics are proven.

Acceptance:

- config validates before apply/persist and refreshes effective capabilities;
- raw cookies/tokens never cross Tauri or enter ordinary TOML/SQLite rows;
- Yande favorite mutation maps add/remove to score 3/2 and is idempotent;
- auth-required/auth-expired states are distinct from network/decode errors;
- live G-05/G-06 evidence is recorded without secrets.

Verify: auth serialization tests, mock mutation tests, clean-profile browser
smoke, then authorized reversible Yande live verification.

### Slice 6 — Yande pools and favorite page gate

Implement public pool metadata/order, pool post pages, archive queue targets,
and `RemoteFavoriteListCapability` only after the authorized read contract is
frozen. Keep pool editing and batch selection deferred.

Acceptance:

- US-Y-10 and US-Y-11 use separate favorite-list and collection capabilities;
- pool ZIP redirects anonymous users to auth and authenticated archives enter
  normal queue/history transitions;
- favorite-page current-user scope, private visibility, ordering, empty state,
  pagination, and refresh-after-mutation are proven;
- G-07 and G-09 receipts are persisted and redacted.

Verify: fixtures plus authorized live checks; a local build cannot substitute
for G-07.

### Slice 7 — release hardening

Run the complete Y-01..Y-09/G-01..G-10 gate, review the user-story matrix,
remove stale scaffolding, verify documentation/code names, and prepare the
release PR. Do not add Pixiv/Twitter behavior, batch workflow, local
bookmarks, creator profiles, or month/quota storage policy in this slice.

## Risks and mitigations

| Risk | Mitigation |
| --- | --- |
| Yande changes response envelopes or popular semantics | Keep fixtures per envelope/period/date and isolate endpoint mapping in the site crate. |
| Favorite read route is not supported | Keep `RemoteFavoriteListCapability` absent and leave Y-07 blocked. |
| Queue writes corrupt or overwrite user files | Validate paths, use temp cache + atomic rename, and test target races. |
| API traits grow around hypothetical sites | Add a capability only when a user story and real adapter need it. |
| Frontend becomes site-aware | Keep site selection/capability rendering data-driven through Tauri DTOs. |

## Implementation commands

- Rust check: `cargo check --workspace`
- Rust tests: `cargo test --workspace`
- Rust format: `cargo fmt --all -- --check`
- Frontend type/build: `bun run typecheck && bun run build`
- Full gate: `make check`
