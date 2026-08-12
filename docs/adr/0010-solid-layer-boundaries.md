# ADR 0010: SOLID boundaries for the site API

- Status: Accepted audit baseline; registry follow-up required
- Date: 2026-08-12

## Context

Dreamland has four relevant boundaries: site-neutral core contracts, the
site composition crate, runtime application services, and concrete site
implementations. The API v1 documents describe capability traits, but the
0.1.0 implementation still uses a small function-based switchboard while the
first real adapters are being proven.

The 0.1.1 release must describe this accurately. A clean crate split is not
the same thing as complete SOLID conformance.

## Audit

| Principle | Current result | Evidence |
| --- | --- | --- |
| Single responsibility | Mostly aligned | `dreamland-core` holds neutral models; site crates decode/map wire data; `dreamland-runtime` owns sessions, persistence, cache, and downloads; Tauri owns IPC/window commands. |
| Open/closed | Partial gap | `dreamland-sites` repeats `match site_id` for each operation. Adding a site requires editing the composition crate. |
| Liskov substitution | Mostly aligned | Active adapters return the same `Post`, `SitePage`, `PoolPage`, and descriptor models; unsupported operations fail explicitly. |
| Interface segregation | Partial gap | `SiteCapabilities` is a flat boolean descriptor and the implemented API is a set of free functions rather than separate object-safe capability ports. |
| Dependency inversion | Partial gap | Runtime calls the composition functions, but the composition crate chooses concrete adapters and bundled defaults directly instead of resolving an injected registry of ports. |

## Decision

Keep the current layers and make their ownership explicit:

```text
core contracts/models
        ▲
site capability ports ← concrete site adapters
        ▲
site registry/composition root
        ▲
runtime application services
        ▲
Tauri controller and typed IPC
```

The current 0.1.1 release does not add a generic dependency-injection
framework or claim that the switchboard is already a trait registry. The
shared Moebooru protocol remains a reusable implementation module, not a
site identity or a runtime service.

The next registry increment must be contract-first and incremental:

1. define object-safe capability ports in `dreamland-core` using typed
   operation inputs and stable `SiteError` outputs;
2. give each concrete site an adapter object that owns its validated defaults;
3. make `dreamland-sites` the only composition root and resolve adapters by
   `SiteId`, with optional capabilities represented by absent ports;
4. migrate Tauri/runtime callers from free-function dispatch to the registry;
5. test descriptor/port agreement and keep unsupported capabilities absent.

This preserves the current working behavior while making the OCP, ISP, and
DIP improvements independently testable. No site-specific branch is added to
`dreamland-core` or `dreamland-runtime` as part of a new adapter.

## Consequences

- 0.1.1 can ship the recent configuration, local-state, cache, and pagination
  work without overstating the maturity of API v1.
- The current switchboard is a known architectural follow-up, not hidden
  accidental coupling.
- A future Pixiv/Twitter implementation must enter through the registry and
  capability ports rather than expanding runtime conditionals.
