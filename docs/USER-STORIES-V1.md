# Dreamland v1 user stories

Status: implementation baseline, 2026-08-12. These stories describe the first
useful desktop workflow. They are the UX acceptance layer above [API v1](API-V1.md)
and the [Yande release gate](YANDE-RELEASE-GATE.md).

"v1" here is the same milestone as [ROADMAP.md](ROADMAP.md)'s "Proposed
0.1.0 gate" and [YANDE-RELEASE-GATE.md](YANDE-RELEASE-GATE.md)'s "the Yande
release" -- one gate, not three. Search (US-Y-04) and remote favorites
(US-Y-09/US-Y-10) ship as part of v0.1.0, scoped to Yande; ROADMAP's Phase
2/3 are for generalizing them to a second site and for local
tags/notes/batch workflows, which remain deferred -- see
[DECISIONS.md](DECISIONS.md#pending-gates).

## Product boundary

The target user explores image-board posts and downloads selected media.
Dreamland starts on Yande, with a compact site selector for future sites.
Local state means saved query definitions, query history, cache state, and
download queue/history. A Yande favorite is remote site state; it is not a
local bookmark or a downloaded-file library.

The v1 download default is `BestAvailable`, stored under the runtime-owned
site path. The queue owns files it creates. It does not scan, import, rename,
or reorganize arbitrary existing files. Batch selection/execution, pool
editing, local post bookmarks, creator profiles, and social actions are not
v1 stories.

## Stories and acceptance

| ID | User story | Happy-path acceptance | API/release mapping |
| --- | --- | --- | --- |
| US-Y-01 | As a user, I open Dreamland and see Yande posts immediately. | The default site is `yandere`; the initial view is a post feed with loading, empty, error, and safe-content states. | `list_sites`, `query_posts`; Y-01, Y-08 |
| US-Y-02 | As a user, I switch sites without losing the exploration model. | The selector lists only registered/enabled sites, refreshes effective capabilities, and never exposes site response JSON. | `list_sites`, `get_site_capabilities`; G-01 |
| US-Y-03 | As a user, I browse Yande popularity. | `PopularPeriod` day/week/month plus an explicit anchor date is visible; infinite scroll continues within the selected date window. | `Feed(Popular { period, anchor_date })`; Y-04, G-04 |
| US-Y-04 | As a user, I search by tags even when I do not know exact spelling. | Suggestions appear after the site accepts the query, show tag category/count metadata, preserve spaces and leading `-`, and selecting a suggestion produces a replayable expression. | `suggest_tags`, `query_posts`; Y-02, Y-03 |
| US-Y-05 | As a user, I save and replay a useful search. | A saved query has a name, complete site/query identity, pin state, and order; replay does not depend on an expired continuation token. | `save_query`, `list_saved_queries`, `delete_saved_query`; local SQLite |
| US-Y-06 | As a user, I inspect a post before downloading. | Detail shows the stable reference, complete typed tags, posting account, rating, dimensions, checksum, source, available media variants, and child/detail state. | `lookup_post`, `hydrate_post`; Y-08 |
| US-Y-07 | As a user, I download one post at the best available quality. | Enqueue returns immediately; progress and terminal state are observable; the worker stages in local cache, atomically renames to the canonical site path, and records history. | `enqueue_download`, `download_history`, `open_download`; G-08 |
| US-Y-08 | As a user, I understand why a download did not replace a file. | If the canonical destination exists, the item ends as `ExistingTarget`; no overwrite, deduplication, silent rename, or arbitrary-file import occurs. | `DownloadStatus::ExistingTarget`; G-08 |
| US-Y-09 | As a user, I sign in and manage a remote favorite. | Browser-session login reports safe auth state; add/remove uses the site action only when authenticated and never sends cookies through React. | `begin_auth`, `complete_auth`, `auth_status`, `execute...`; Y-05, Y-06 |
| US-Y-10 | As a user, I open my remote favorite page. | After authorized read semantics are verified, the page loads current-user favorites with loading/empty/error/auth states and refreshes after mutation. | `list_remote_favorites`; Y-07, G-07 |
| US-Y-11 | As a user, I browse a Yande pool and request its ZIP. | Public pool metadata and ordered posts are visible; ZIP is a distinct archive queue target and requires site auth when Yande redirects anonymous users. | `list_remote_collections`, `list_remote_collection_posts`, collection download; Y-09, G-09 |
| US-Y-12 | As a user, I inspect download history and open a completed file. | History shows outcome, path, checksum, immutable metadata snapshot, and failure reason; opening a file is runtime-owned and does not expose arbitrary paths to React. | `download_history`, `open_download`; G-08 |
| US-Y-13 | As a user, I configure the runtime safely. | I can set download directory, quality, concurrency, cache, logging, and site non-secret options; validation happens before apply/persist. | `load_config`, `validate_site_config`, `save_config`; G-01 |
| US-Y-14 | As a user, I follow an author tag from a post and return to my previous exploration. | Opening a post adds a detail entry; selecting its author tag adds a search entry; Back returns search → detail → the originating feed with its site, popular period/date, loaded results, selected position, and scroll context. Forward retraces the same path. | React navigation history + TanStack Query cache; no new site API |

### Daily workflow stories

These are cross-feature habits the app must preserve on every route; they are
not additional site capabilities.

| ID | User story | Acceptance |
| --- | --- | --- |
| D-01 | As a user, I refresh or retry the screen I am using. | Refresh keeps the current route, site, search expression, popular period/date, pool selection, and content policy; loading replaces stale content with a calm skeleton and retry returns to the same route. |
| D-02 | As a user, I open a pool, inspect posts, and return to the pool list. | Pool list → pool detail → Back preserves the pool search and list position; pool-post pagination is independent from pool-list pagination and does not create history entries. |
| D-03 | As a user, I download while continuing to browse. | Enqueue never changes the current route; one top-right notification reports queued/completed/existing/failed state, with no duplicate inline banner; Downloads remains the durable history and opens completed files through the runtime. |
| D-04 | As a user, I change sites during exploration. | The route changes to the selected site’s equivalent view only when that capability exists; unsupported tabs disappear, old site results/detail state are cleared, and Back never returns to a stale site response. |
| D-05 | As a user, I use keyboard and window navigation. | Escape closes detail as Back; Left/Right changes the detail carousel without adding history entries; platform Back/Forward and their keyboard equivalents retrace route entries. |
| D-06 | As a user, I reopen Dreamland after leaving it. | The app restores the last safe route and site from local preferences; route intent is replayed when remote data is cold, while exact scroll and loaded-page state remain session-local. |

## Failure and boundary coverage

“Evil” cases are hostile or untrusted inputs; edge cases are unusual but
valid states. Every story that crosses the runtime boundary must preserve the
following behavior.

| Case | Expected behavior | Covered by |
| --- | --- | --- |
| Unknown, disabled, unhealthy, or unsupported site | Return a stable runtime error or omit the unavailable capability; never fall back silently. | US-Y-02, G-01 |
| Malformed, legacy, or v2 Yande post response | Decode through the site adapter, normalize only valid fields, and return a safe decode error when the envelope is invalid. | US-Y-01/04/06, G-02 |
| Empty suggestions, empty page, or no matching tags | Show an explicit empty state; do not treat it as a transport failure or fabricate tags. | US-Y-04, G-02/G-03 |
| Rate limit, timeout, cancellation, or stale response | Map to stable retryable/cancelled errors; discard late results from an older query/session. | US-Y-03/04, G-01/G-02/G-10 |
| Popular query receives `page`/`limit` | Preserve the selected date window while continuing the score-ranked `post.json` query; stop when a page is shorter than the requested size. | US-Y-03, G-04 |
| Anonymous favorite mutation/list/ZIP | Return `auth_required` or `auth_expired`; do not retry with guessed routes. | US-Y-09/10/11, G-05/G-06/G-07/G-09 |
| Favorite read mechanism remains unverified | Keep US-Y-10 blocked in the release gate; the UI may render a capability placeholder but no endpoint is hardcoded. | Y-07, G-07 |
| Site returns a URL with an unsafe scheme or redirect | Runtime validates the resolved media/archive URL and rejects non-allowed schemes or untrusted redirects. | US-Y-07/11, G-08/G-09 |
| Post/media ID, checksum, tag, or account is missing | Keep optional metadata unknown; use stable site/post identity and safe fallback names. | US-Y-06/07, G-04/G-08 |
| Destination exists before download or rename | End as `ExistingTarget`; never overwrite, merge, deduplicate, or invent a new filename. | US-Y-08, G-08 |
| Path traversal, reserved name, excessive component, or Unicode oddity | Normalize path components and keep all writes beneath the configured directory. | US-Y-07/13, G-08 |
| Worker crash or app restart | Durable SQLite state makes queued/running work recoverable; temporary cache files are reconciled by the runtime, not exposed as library files. | US-Y-07/12, G-08 |
| User changes config/auth while a query/download runs | Existing operation keeps its bound context or is cancelled explicitly; stale capabilities and continuation tokens cannot be reused. | US-Y-02/09/13, G-01/G-10 |
| Detail-to-tag navigation loses its source feed | Keep a replayable feed/search intent and live feed snapshot in the navigation entry; never depend on an expired continuation token to reconstruct Back. | US-Y-03/04/06/14 |
| Refresh/retry is issued while a route is loading | Keep the route and request fingerprint stable; cancel the obsolete operation and suppress late results instead of resetting to latest. | D-01, G-01/G-10 |
| A site lacks the current view capability | Remove or disable only that route action, clear site-bound data, and show a stable unsupported state; never render another site’s posts under the new site name. | D-04, G-01 |
| App restarts with an old route or expired cursor | Restore only validated route intent and start a new query session; never persist or reuse a remote continuation cursor as navigation state. | D-06, G-02/G-10 |

## Deliberate non-stories

- No local bookmark collection for posts.
- No arbitrary existing-file scan, import, library indexing, or reorganization.
- No batch-selection API; multiple explicit enqueue calls may be added later
  after the single-download lifecycle is proven.
- No tags or posting-account names in canonical directory keys.
- No generic creator-profile screen in the Yande release.
- No route-history persistence across application restart; the query intent may
  be replayed, but the exact scroll position and loaded-page snapshot are
  session-local.
- No assumption that a pool ZIP is equivalent to selecting and downloading
  individual cards.

## Approval checklist

- [ ] The default Yande feed, selector, search, detail, download, auth,
      favorite, pool, history, and configuration stories are accepted.
- [ ] Y-07 remains explicitly blocked until authorized favorite-list semantics
      are verified.
- [ ] The existing-target and restart/recovery rules are accepted.
- [ ] “One month space” is resolved before adding retention or storage-quota
      behavior; v1 currently specifies neither month directories nor a free-
      space reservation. The phrase's origin is not recorded anywhere in
      these docs -- whoever introduced it should either define it here or
      confirm it can be dropped as a stale reference.
