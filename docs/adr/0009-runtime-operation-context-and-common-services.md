# ADR 0009: Runtime operation context and common services

- Status: Proposed
- Date: 2026-08-11

## Context

The API v1 documents describe a trait-based site boundary with a request
context, typed authentication, runtime-owned cache, and structured errors.
The current implementation is one step earlier: `dreamland-core` contains
the common data types, `dreamland-sites` dispatches through site-ID matches,
and Tauri still coordinates the Yande browser session. Runtime media and
download state are owned correctly, but several concerns are still packed
into large Rust modules.

Go's `context.Context` is useful here as a lifecycle idea, but not as a
general dependency-injection bag. Auth, cache, logging, configuration, and
HTTP clients have different ownership and security rules.

## Decision proposed

Adopt small, typed runtime services and an explicit operation context during
the trait-contract migration:

```text
dreamland-runtime
├── request.rs   operation id, cancellation, deadline, site, session handle
├── auth.rs      opaque site-session storage and auth lifecycle
├── cache.rs     typed cache keys, staging, lookup, invalidation
├── logging.rs   structured redacted runtime events
└── config.rs    app-wide settings and validated site extensions
```

`RequestContext` carries request lifecycle data only: operation identity,
cancellation/deadline, site identity, and an opaque session handle. It must
not carry arbitrary key/value data, raw cookies, filesystem paths, logger
instances, or mutable configuration.

The service boundaries are:

- `AuthSessionStore` keeps cookies/tokens private and exposes only status or
  an opaque site-scoped handle. The Tauri WebView remains a browser bridge;
  it does not become the owner of site cookie semantics.
- `CacheStore` owns detail-image cache, normalized metadata references, and
  temporary download staging. Cache keys are derived from `PostRef` and
  media variant, never from a frontend-provided URL or path.
- `Logger` accepts typed events and performs redaction centrally. Raw cookie
  headers and private response bodies are never log fields.
- `AppConfig` remains the owner of app-wide settings. Site endpoint, cookie,
  and protocol settings remain in each concrete site adapter and are passed
  into an adapter at construction.

The first mechanical step is to split runtime media/download behavior from
the runtime crate root without changing its public API. Larger state and
adapter migrations follow only when the corresponding typed boundary has at
least two real consumers.

## Consequences

- Request cancellation and stale-result suppression can cross the site
  boundary without hiding dependencies in global state.
- Site authentication can be generalized without exposing Yande cookies to
  Tauri commands or the frontend.
- Cache and logging policies become testable ports rather than duplicated
  filesystem and HTTP concerns.
- The migration adds no empty abstraction today: the current media split is
  mechanical, while auth/cache/logging ports remain proposed until the
  adapter registry and core traits are implemented.

## Migration order

1. Define the actual core capability traits and `RequestContext`.
2. Replace the `dreamland-sites` switchboard with a registry of adapters.
3. Move Yande cookie interpretation and auth mutation behind site auth plus
   the runtime session store.
4. Normalize site/runtime errors before the Tauri boundary.
5. Wrap the existing detail cache, download staging, and logging in typed
   runtime services.
6. Split SQLite state into schema, queue/history, and repository modules.
