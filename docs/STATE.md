# Local state contract

Dreamland is local-first for user intent and download work, but not for remote
account state. The runtime owns the boundary between durable SQLite records,
disposable files, user downloads, and site state.

## Spring-style layer mapping

| Layer | Dreamland owner | Rule |
| --- | --- | --- |
| Controller | `src-tauri` commands and typed frontend IPC | accepts application inputs and returns safe DTOs |
| Application service | `dreamland-runtime` workers and use cases | owns lifecycle, policy, transactions, and orchestration |
| Repository | `StateStore` backed by `dreamland-state::SqliteStateRepository` | owns persistence calls; sites and React never access SQLite |
| Model | `dreamland-core` plus local queue/history records | carries validated domain data, not HTTP or UI concerns |
| Infrastructure | `dreamland-state` | owns SQLite connection policy and current schema |

The controller/service/repository names describe ownership, not a new web
framework. The Tauri shell is the controller; there is no HTTP MVC server in
Dreamland.

## SQLite choice and ownership

The persistence dependency is [`rusqlite`](https://docs.rs/rusqlite/0.32),
version `0.32` with the `bundled` feature. Bundling SQLite keeps macOS and
Windows builds independent of an installed system library. Dreamland does not
use an ORM or a second database abstraction.

`dreamland-state` is the specified persistence crate. Its public API is
small:

- `SqliteStateRepository::open(path)` creates the parent directory, initializes
  the current schema, and fixes the busy timeout;
- `SqliteStateRepository::path()` exposes the runtime-owned database path;
- `SqliteStateRepository::connection()` provides a configured connection to
  the runtime repository implementation;
- `initialize_schema()` creates the current schema idempotently.

`dreamland-runtime` remains the application service and repository API for
queue/history operations. The SQLite crate does not know about site adapters,
Tauri, authentication, or media downloads.

## Stored data

SQLite stores:

- `download_queue` and `download_history` for image jobs and terminal results;
- `archive_queue` and `archive_history` for pool archive jobs and results;
- `saved_queries` for local replayable search definitions, pin state, and order.

Rows may contain normalized metadata snapshots, validated source references,
target roots, retry counters, and error states. SQLite does not store passwords,
raw cookies, auth tokens, detail-image bytes, the browser route stack, or the
configured download library. Remote favorites remain remote site state.

## Transaction and recovery rules

Queue claims and terminal transitions are transactions. A restart changes only
runtime-owned `Running` rows back to `Queued`; it never scans or rewrites
arbitrary user files. The worker checks the final target immediately before
atomic rename, so an existing completed file remains authoritative.

The configured download library is the durable media store. Detail images are
reusable cache entries, and a download promotes a copy into the library without
removing a cache path that an open detail view may still be using. Detail lookup
checks the configured library first and the detail cache second, so a stale
detail entry cannot mask an already-downloaded file. Cache cleanup does not
invalidate already-downloaded media. `detail-staging` and `downloads` contain
only in-flight temporary files. Cache cleanup is refused while either queue
contains `Queued` or `Running` work.

History records attempts, not just unique files. A later request for a post whose
canonical target already exists becomes `ExistingTarget`; the earlier
`Completed` record remains useful evidence and is not silently merged. UI
summaries count unique target paths, while the row vocabulary is `downloaded`
for a new file and `on disk` for an existing file kept without overwrite.
Successful download rows use the local library file for their thumbnail first,
then fall back to the remote preview. Selecting that thumbnail opens the same
detail inspector as a feed card; if post metadata is still hydrating, the
inspector retries once the full-image URL arrives.
If a cached frontend route outlives the bounded runtime post cache, enqueueing
performs one authoritative site lookup by site and post id before resolving the
download URL; scrolling does not turn the runtime cache into an unbounded
history of every rendered post.

The current schema is idempotent and tested in `dreamland-state`. New tables or
columns must land there before a runtime repository method consumes them.

## Common lifecycle hooks

Spring lifecycle/interceptor behavior is useful as a constraint, not as a
reason to add a dependency-injection container. Shared operations follow this
sequence:

```text
load config
  → validate common policy and site extension
  → create operation context
  → before-dispatch hooks: auth, cache, redacted logging
  → site dispatch and normalization
  → persist local state when applicable
  → after-success or on-error hooks
  → publish a safe frontend DTO
  → finally: release operation/session resources
```

Hooks are ordered, typed, runtime-owned, and operation-scoped. They may enforce
authentication, cache policy, cancellation, redacted logging, and stable error
mapping. Site adapters may validate and consume their own extension section,
but cannot bypass common hooks or write SQLite directly. Hook context must not
carry raw cookies, arbitrary key/value dependencies, frontend paths, or mutable
global configuration.

The current concrete common behaviors are shared Moebooru retry/error mapping,
runtime-owned queue/cache handling, transactional state transitions, and crash
recovery. Add a hook only when at least two real operations share the behavior.
