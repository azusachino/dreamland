# ADR 0010: SOLID boundaries for the site API

- Status: Accepted and implemented in 0.1.1
- Date: 2026-08-12

## Context

Dreamland has four relevant boundaries: site-neutral core contracts, the
site composition crate, runtime application services, and concrete site
implementations. The API v1 documents describe capability traits, and the
0.1.1 implementation now applies TOML site enablement through the registry and
uses a site-bound `SiteSession` for authenticated capability calls.

The 0.1.1 release must describe this accurately. A clean crate split is not
the same thing as complete SOLID conformance, so the provisional switchboard
must be replaced before the release is considered complete.

## Audit

| Principle | Current result | Evidence |
| --- | --- | --- |
| Single responsibility | Mostly aligned | `dreamland-core` holds neutral models; site crates decode/map wire data; `dreamland-runtime` owns sessions, persistence, cache, and downloads; Tauri owns IPC/window commands. |
| Open/closed | Aligned at the runtime boundary | `SiteRegistry` resolves `SiteAdapter` objects by identity; Tauri/runtime no longer match site IDs or import concrete adapters. The composition root is the intentional registration point. |
| Liskov substitution | Mostly aligned | Active adapters return the same `Post`, `SitePage`, `PoolPage`, and descriptor models; unsupported operations fail explicitly. |
| Interface segregation | Aligned for shipped capabilities | Optional features are separate object-safe ports; unsupported operations are absent rather than fake methods. Descriptor flags are validated against the ports. |
| Dependency inversion | Aligned at the application boundary | Runtime/Tauri depend on `SiteAdapter` ports and `SiteRegistry`; only the composition root depends on concrete adapter crates and their defaults. |

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

The 0.1.1 implementation adds no generic dependency-injection framework. The
shared Moebooru protocol remains a reusable implementation module, not a site
identity or a runtime service.

The registry implementation follows this contract-first standard:

1. define object-safe capability ports in `dreamland-core` using typed
   operation inputs and stable `SiteError` outputs;
2. give each concrete site an adapter object that owns its validated defaults;
3. make `dreamland-sites` the only composition root and resolve adapters by
   `SiteId`, with optional capabilities represented by absent ports;
4. route Tauri/runtime callers through the registry; keep browser-session
   lifecycle and the default product site's user-flow selection in the
   composition/controller boundary rather than in core or generic runtime
   services;
5. test descriptor/port agreement and keep unsupported capabilities absent.

This preserves the current working behavior while making the OCP, ISP, and
DIP improvements independently testable. No site-specific branch is added to
`dreamland-core` or `dreamland-runtime` when another adapter is added. A future
authenticated site may require its own login-window command wiring until the
browser-session lifecycle is generalized.

## Consequences

- 0.1.1 ships the recent configuration, local-state, cache, pagination, and
  capability-registry work behind one standard.
- A future Pixiv/Twitter implementation must enter through the registry and
  capability ports rather than expanding runtime conditionals.
