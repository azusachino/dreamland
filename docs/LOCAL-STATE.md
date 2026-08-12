# Local state contract

Dreamland is local-first for user intent and download work, but not for remote
account state. The runtime owns the boundary between durable SQLite records,
disposable files, user downloads, and site state.

## Spring-style layer mapping

| Layer | Dreamland owner | Rule |
| --- | --- | --- |
| Controller | `src-tauri` commands and typed frontend IPC | accepts application inputs and returns safe DTOs |
| Application service | `dreamland-runtime` workers and use cases | owns lifecycle, policy, transactions, and orchestration |
| Repository | `LocalStateStore` backed by `dreamland-local-state::SqliteStateRepository` | owns local persistence calls; sites and React never access SQLite |
| Model | `dreamland-core` plus local queue/history records | carries validated domain data, not HTTP or UI concerns |
| Infrastructure | `dreamland-local-state` | owns SQLite connection policy, schema, and migrations |

The controller/service/repository names describe ownership, not a new web
framework. The Tauri shell is the controller; there is no HTTP MVC server in
Dreamland.

## SQLite choice and ownership

The persistence dependency is [`rusqlite`](https://docs.rs/rusqlite/0.32),
version `0.32` with the `bundled` feature. Bundling SQLite keeps macOS and
Windows builds independent of an installed system library. Dreamland does not
use an ORM or a second database abstraction.

`dreamland-local-state` is the specified persistence crate. Its public API is
small:

- `SqliteStateRepository::open(path)` creates the parent directory, applies
  migrations, and fixes the busy timeout;
- `SqliteStateRepository::path()` exposes the runtime-owned database path;
- `SqliteStateRepository::connection()` provides a configured connection to
  the runtime repository implementation;
- `migrate()` applies idempotent, forward-only schema changes and records the
  current version in `PRAGMA user_version`.

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

Schema migrations are forward-only, idempotent, and tested in
`dreamland-local-state`. New tables or columns must land there before a
runtime repository method consumes them.

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
