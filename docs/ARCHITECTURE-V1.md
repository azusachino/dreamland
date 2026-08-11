# Dreamland v1 architecture

Status: design gate, 2026-08-11. This document turns the API and user stories
into ownership, workflow, dataflow, and failure-boundary rules. It does not
authorize implementation until the API and release gates are approved.

## Component ownership

```text
React UI
  │ application DTOs only
  ▼
Tauri command boundary
  │ validation, operation IDs, cancellation
  ▼
Dreamland runtime
  ├── dreamland-sites composition root → selected SiteAdapter → remote HTTP
  ├── Auth/session bridge → browser flow + secret store
  ├── SQLite LocalStateStore → saved queries, history, queue, cache metadata
  ├── Download worker → site media/archive resolution
  │                    → temporary local cache → atomic final rename
  └── filesystem opener → runtime-owned completed download path
```

Ownership is strict:

| Concern | Owner | Never owned by |
| --- | --- | --- |
| Rendering, selector, query editor, card/detail state | React | site adapter or filesystem |
| Site HTTP, decoding, normalization, capability negotiation | SiteAdapter/runtime | React |
| Auth cookies, tokens, session handles | browser bridge/secret store/runtime | React, ordinary TOML, SQLite rows |
| Saved query definitions and query history | SQLite LocalStateStore | remote site |
| Remote favorites, pools, pool ZIP route | the selected site through capabilities | local saved-query state |
| Queue state, metadata snapshots, terminal outcomes | SQLite + runtime worker | remote site or React |
| Temp files and final paths | runtime download worker | React or site response JSON |

## Core dataflow

Every remote read follows the same path:

```text
user intent
  → React application input
  → Tauri command validation
  → runtime operation/session
  → SiteAdapter request DTO
  → site response
  → site DTO decoder
  → common Post/Collection models
  → runtime safety/content filters
  → Tauri result + local cache metadata
  → React view state
```

`dreamland-sites` is the composition root: it assembles the active registered
site crates and keeps descriptor-only Pixiv/Twitter skeletons separate from
the active registry. It is the only site-discovery dependency the Tauri shell
needs. The adapter is the only place that knows Yande request syntax,
response envelopes, cookie names, popular endpoint families, pool ZIP routes,
or a future site's equivalent. The runtime is the only place that knows
operation lifetimes, safe URL policy, SQLite, cache paths, canonical filenames,
and filesystem access.

## Workflow state machines

### Query

```text
Idle → Starting → Loading → Ready
                    │          ├── Continue → Loading
                    │          └── Refresh → Starting
                    ├── Empty
                    ├── AuthRequired / Unsupported
                    ├── RateLimited / RetryableError
                    ├── Cancelled
                    └── DecodeError / RemoteError
```

Each query session owns the site ID, normalized request fingerprint, auth
identity, pagination state, cancellation, and seen post references. A late
response is discarded when its session or operation is no longer current.
Popular day/week/month windows enter `Ready` with no continuation when the
site advertises `FixedWindow`.

### Download

```text
Requested → Queued → Resolving → Streaming → Staged → Committing → Completed
                      │             │          │          │
                      ├─ AuthRequired       ├─ Cancelled ├─ ExistingTarget
                      ├─ Unsupported        ├─ Network   └─ StorageError
                      └─ ResolveError
```

`enqueue_download` commits a durable queue row and returns immediately. The
worker resolves the requested variant or collection archive, validates the
resolved URL, streams into a temporary file in the local cache, checks the
canonical target again, then atomically renames. `ExistingTarget` is terminal
for that attempt. Retry creates a new attempt without overwriting the old
terminal outcome.

### Authentication

```text
Anonymous → ChallengeStarted → BrowserComplete → Authenticated
    │                              │                 │
    └──── auth_required ◄──────────┘                 ├─ Expired → Refreshing
                                                     └─ Logout → Anonymous
```

React receives only safe auth status and challenge metadata. The browser bridge
imports declared cookie domains into the secret/session boundary. Favorite,
favorite-list, and pool-ZIP operations ask the runtime for an internal session
handle and never receive raw cookie material.

## Feature-to-flow bindings

| User-visible action | Required capability/command | Site-specific part | Runtime-owned part |
| --- | --- | --- | --- |
| Tag search | `PostQueryCapability` + `TagSuggestionCapability` | Yande tag expression and `/tag` mapping | query session, safe filtering, stale-result suppression |
| Popular feed | `PostQueryCapability` | day/week/month endpoint and fixed-window/date semantics | feed state and no-false-pagination UI data |
| Post detail | lookup/detail/children capabilities as advertised | response hydration and media variants | partial state, caching, common model |
| Favorite add/remove | `RemoteFavoriteCapability` + `SiteAuth` | Yande score 3/2 mutation | auth gating, idempotency, race protection |
| Favorite page | `RemoteFavoriteListCapability` + `SiteAuth` | verified current-user read route/query | collection session, loading/empty/error states |
| Pool browse | `RemoteCollectionCapability` | Yande pool metadata and ordered posts | pagination/session and common collection model |
| Pool ZIP | `CollectionDownloadCapability` | Yande `/pool/zip/:id` and auth behavior | archive target, queue, path, history |
| Single download | enqueue/download worker | media resolution and referer requirements | best quality selection, URL validation, atomic write |
| Saved query | `LocalStateStore` | stored site expression remains opaque | pin/order/history and replay |
| Settings | config commands + `SiteConfigSchema` | site extension fields | validation, secret references, persistence, capability refresh |

Pools are not part of the required site core. Yande binds them by advertising
`RemoteCollectionCapability` and `CollectionDownloadCapability`; Pixiv can
advertise creator/children/media/bookmark capabilities without implementing
pool methods. The UI renders only advertised capabilities.

## Data ownership and persistence

SQLite stores:

- `saved_queries`: complete site/query intent, name, pin, and order;
- `query_history`: recent replayable intents, never raw continuation tokens;
- `download_queue` / `download_history`: target, attempt state, path,
  checksum, error, and immutable normalized metadata snapshot;
- `site_cache`: expiring normalized metadata/cache references, never remote
  truth and never secret material.

The final file layout is:

```text
<directory>/<site>/posts/<post-id>_<checksum>.<extension>
<directory>/<site>/pools/pool-<pool-id>_<safe-pool-name>.zip
```

If the checksum is absent, the post ID remains the identity fallback. Tags,
artist tags, and posting-account names remain metadata/search fields, not
directory keys. Date remains a filter, not the primary organization.

The current v1 does not define month directories, free-space reservations, or
retention quotas. Those are separate decisions; the phrase “one month space”
must not silently become a storage policy. Its origin is not recorded in any
Dreamland doc -- see the open checklist item in
[USER-STORIES-V1.md](USER-STORIES-V1.md#approval-checklist).

## Security and evil-case boundaries

- React cannot submit a URL, referer, cookie, token, arbitrary path, or site
  DTO as authority.
- The runtime accepts only allowed media/archive URL schemes and validates
  redirects before streaming.
- Canonical path components are normalized for traversal, reserved names,
  Unicode oddities, and length; all writes stay below the configured root.
- Existing targets abort before write and immediately before rename.
- Malformed remote payloads become safe decode errors; unknown fields are not
  silently promoted to common fields.
- Auth failures are distinct from network/decode failures and do not leak
  cookies or credentials in errors/logs.
- A site capability that is absent is not represented by a fake successful
  method. The frontend cannot invent action or collection IDs.
- SQLite queue transitions are transactional; crash recovery reconciles only
  runtime-owned queue/cache files, never arbitrary user files.

## Verification coverage

Architecture verification should be layered:

1. **Adapter fixtures:** Yande array/v2 envelopes, tag metadata, popular fixed
   windows, pool records, media variants, auth/error mappings.
2. **Runtime contract tests:** capability consistency, safe URL/path handling,
   query cancellation, stale responses, auth gating, queue durability,
   existing-target abort, atomic rename, and stable errors.
3. **Tauri serialization tests:** only application DTOs cross the boundary;
   cookies, URLs, paths, and site DTOs are rejected.
4. **UI workflow tests:** each US-Y story's loading/empty/error/auth state and
   no-false-pagination behavior.
5. **Authorized live checks:** favorite mutation reversibility, favorite-list
   semantics, and pool ZIP auth/content behavior. Live checks are separate from
   fixtures and cannot be replaced by a local build pass.

The release is not ready while Y-07/G-07 remains unverified. No architecture
diagram or generic capability flag changes that external evidence requirement.
Pixiv and Twitter are workspace skeletons only. Their descriptors are useful
for planning and capability review, but `descriptors()` does not expose them
to the runtime or UI until real adapter methods, auth boundaries, fixtures,
and release evidence exist. A descriptor with all capabilities disabled is not
a substitute for an implementation.
