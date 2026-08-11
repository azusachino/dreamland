# Yande release feature gate

Status: design gate, 2026-08-10. A feature is not considered shipped because
its UI or site method exists. It must have a contract, fixture coverage,
and the required live/auth evidence recorded below.

## Release objective

Ship Dreamland’s first useful Yande experience around five user journeys:

1. discover posts by tag;
2. browse the day/week/month popularity windows;
3. sign in through a Yande browser session; and
4. view, add, and remove remote favorites; and
5. browse pools and request a pool ZIP archive.

Saved query definitions, download history, and cached metadata remain
Dreamland-owned data. They must not be confused with Yande’s remote favorite
state.

## Scope gate

### Must ship

| ID | Feature | Acceptance boundary |
| --- | --- | --- |
| Y-01 | Yande site registration | Site descriptor, stable `yandere` identity, safe default, and config entry are available through the runtime. |
| Y-02 | Tag search | Site tag expression is encoded and queried through the paged post API; page size, safe filtering, empty results, and both response envelopes are handled. Per [MOEBOORU-UX-LEARNINGS.md](MOEBOORU-UX-LEARNINGS.md), this includes round-tripping negative-tag (`-tag`) prefixes and space-tokenized multi-term queries -- not yet in G-02's fixture list below. |
| Y-03 | Tag suggestions | Live Yande `/tag.json` autocomplete returns name/count/type/ambiguity metadata and handles empty/error responses; pinned MoeLoaderP `/tag.xml` behavior remains separate source evidence. |
| Y-04 | Popular by day/week/month | `PopularPeriod` plus an explicit anchor date maps to a score-ranked `/post.json` query: day uses one date, week uses the Monday-Sunday window, and month uses the calendar-month window. The selected window is page-number paged for infinite scroll. |
| Y-05 | Browser-session login | The user can open/import Yande login state; the runtime detects the current `user_info` cookie (and accepts legacy `user_id`), reports auth state, and never exposes raw cookies or credentials to React/TOML. |
| Y-06 | Add/remove remote favorite | Authenticated `RemoteFavoriteCapability` maps Yande favorite add to score 3 and remove to score 2; unauthenticated calls return `auth_required`. |
| Y-07 | Remote favorite page | An authenticated user can open a dedicated favorite page through `RemoteFavoriteListCapability`, load the user’s remote favorites, paginate when the verified site mechanism supports it, and see loading/empty/error/auth-required states. The exact Yande list mechanism is a release blocker until verified. |
| Y-08 | Post cards and download | Normalized identity, tags, rating, dimensions, preview/sample/original variants, safe filtering, and reference-based download work for all Yande result modes. Per [REFERENCE-LEARNINGS.md](REFERENCE-LEARNINGS.md), Yande also resolves an existing image by checksum via `md5:<hash>`; whether the download path uses this for dedupe is not yet decided or gated. |
| Y-09 | Pool browse and ZIP | Public pool metadata and ordered posts are browseable; an authenticated user can request the site ZIP route through `CollectionDownloadCapability`; the runtime owns the destination and history. |

### Should ship

- Favorite state is reflected on cards after add/remove and after refresh.
- Re-authentication and expired-session recovery return the user to the
  interrupted favorite operation without duplicating it.
- Popular pages label the selected date/window and continue loading while the
  selected score-ranked query returns full pages.
- Favorite-page refresh and add/remove are race-safe: a stale response cannot
  overwrite a newer auth or favorite state.

### Deferred

- pool editing;
- tag mutation, post editing, voting other than the favorite mapping;
- upload, delete, moderation, notes, and translations;
- batch-download workflow and site-wide collection abstractions beyond the
  first required favorite page;
- assuming Yande's raw `/post/popular_by_*` endpoints provide pagination; those
  fixed 40-item endpoints are not the path used by Dreamland's scrollable
  popular feed.

## Favorite-page decision gate

The favorite page is required, but the implementation must not guess its
backend route. The live evidence currently establishes:

- `/post/vote.json` is the web mutation endpoint (asserted from MoeLoaderP
  research, undated and unverified against a live account -- this claim has
  not had the same authorized-verification treatment as the favorite-list
  read below, even though it is the riskier operation because it mutates a
  real account; G-06 should not be marked `PASS` on mock coverage alone);
- `post.json` accepts site tag expressions and page/limit parameters;
- `/favorites` currently returns 404 without an authenticated route;
- the candidate read path is a user-scoped favorite query such as
  `vote:3:<username> order:vote`, but current-user identity, private-favorite visibility,
  ordering, and pagination still require authenticated verification.

Therefore Y-07 is `BLOCKED` until one of these is proven with an authorized
session:

1. Yande’s authenticated `post.json` favorite query is the supported read
   contract, including current-user identity and private visibility; or
2. Yande exposes another authenticated favorite-list endpoint that returns
   normalized posts and a stable continuation model.

The UI may be designed against `RemoteFavoriteListCapability`, but no endpoint should
be hardcoded into the frontend.

## Verification gate

The release gate is passed only when every Must feature has the required local
evidence and every external/authenticated behavior has authorized live
evidence. “Build passes” alone is not a release decision.

| Gate | Verification | Evidence required | Status |
| --- | --- | --- | --- |
| G-01 | Site contract | Fake site tests descriptor, capability negotiation, safe defaults, operation IDs, cancellation, and unsupported-capability errors. | Pending API approval |
| G-02 | Tag search | Fixture tests for legacy array and v2 envelope, tag encoding, page/limit, safe mode, empty page, malformed response, and rate-limit/error mapping. | Pending |
| G-03 | Tag suggestions | Fixture tests for tag name/count/category mapping, empty response, cancellation, and remote error. | Pending |
| G-04 | Popular modes | Fixtures assert `/post.json` mapping, day/week/month date expressions, Monday-first week boundaries, `order:score`, page/limit continuation, short-page termination, and infinite-scroll UI. Raw `/post/popular_by_*` fixed-window behavior remains documented but is not used for this feed. | Adapter fixtures pass; live page-2 parity verified for week; tab-switch rebind fix passes typecheck/build, but interactive browser verification remains pending |
| G-05 | Login/auth | Auth-state tests with absent/valid/expired cookie; Tauri serialization test proves cookies never cross the command boundary; manual browser-session smoke on a clean profile. | Manual auth smoke pending |
| G-06 | Favorite mutation | Mock tests assert score 3/2 mapping and idempotent state handling; authorized live smoke adds then removes one explicitly selected test post and verifies the remote result. | Live write authorization pending |
| G-07 | Favorite page | Authorized live read smoke proves current-user identity, private visibility, ordering, empty state, pagination/continuation, and refresh after mutation; fixture tests cover all UI states. | **BLOCKED: read mechanism unverified** |
| G-08 | Download | Fixture-to-runtime tests prove the frontend supplies only a `DownloadTarget`; async queue durability, temporary-cache staging, atomic rename, existing-target abort, URL scheme, referer, safe filtering, and local path ownership are enforced. | Pending |
| G-09 | Pool ZIP | Fixtures cover pool metadata/order and archive target mapping; authorized live smoke verifies the ZIP route, auth redirect, content type, and terminal history state without storing the archive in fixtures. | Authenticated live smoke pending |
| G-10 | Regression gate | `make check` plus site/runtime tests pass. | Verified 2026-08-12: `make check` passed, including TypeScript, Vite build, Rust tests, formatting, and Nix checks. Not hermetic -- `flake.nix` provides `bun` but not `typescript`; a fresh shell needs `bun install` (network) before `tsc` exists. |

## Live verification rules

Live verification is deliberately separate from CI fixtures:

- It requires explicit authorization for the Yande account and selected test
  post; no credentials or cookies may be placed in logs, fixtures, TOML, or
  commits.
- Favorite mutation must be reversible: add, observe success, remove, and
  observe the final state.
- The verification receipt records endpoint family, date/time, site git
  SHA, result counts/statuses, and redacted error codes—not account secrets or
  raw response bodies.
- A failed or incomplete live check leaves the gate `BLOCKED`; local tests do
  not upgrade it to `PASS`.

## Ship decision

The Yande release is `READY` only when Y-01 through Y-09 are implemented and
G-01 through G-10 are either `PASS` or have an explicitly approved non-live
exception. At the current design checkpoint the release is **not ready**:

- the feature scope is now explicit;
- popular endpoint behavior is source/live-backed;
- favorite mutation mapping is source-backed;
- favorite-page read semantics remain unverified;
- implementation and authorized live verification have not started.
