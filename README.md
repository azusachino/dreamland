# Dreamland

> A calm, native image-board browser for finding, inspecting, and keeping the
> art you actually want.

[![status: 0.1.1 release candidate](https://img.shields.io/badge/status-0.1.1%20release%20candidate-557c86)](https://github.com/azusachino/dreamland/issues/5)
[![ci](https://github.com/azusachino/dreamland/actions/workflows/ci.yml/badge.svg)](https://github.com/azusachino/dreamland/actions/workflows/ci.yml)
[![targets: macOS and Windows](https://img.shields.io/badge/targets-macOS%20%7C%20Windows-557c86)](docs/adr/0001-platform-and-toolchain.md)
[![Tauri 2](https://img.shields.io/badge/Tauri-2-24c8db)](https://v2.tauri.app/)
[![React 19](https://img.shields.io/badge/React-19-61dafb)](https://react.dev/)
[![Rust](https://img.shields.io/badge/runtime-Rust-b7410e)](https://www.rust-lang.org/)

Dreamland is a cross-platform desktop gallery built around one simple loop:

```text
choose a site  →  explore a feed  →  open the artwork  →  keep it locally
```

The frontend is React and TypeScript. The Rust runtime owns remote access,
site capabilities, authentication state, local persistence, cache lifecycles,
and downloads. That separation keeps the UI expressive without letting every
new site leak transport details into the application.

## The 0.1.1 surface

| Explore | Inspect | Keep | Configure |
| --- | --- | --- | --- |
| latest, popular, pools, tag search, suggestions | full-quality detail view, related tags, keyboard navigation | async download queue, duplicate-safe writes, history, local file opening | TOML settings, per-site extensions, XDG paths, bounded retries, cache cleanup |

The current release candidate is intentionally focused:

- **yandere** is the default release path, including popular day/week/month
  views, exact tag queries, tag suggestions, pools, pool ZIP downloads,
  favorites, and authenticated account state.
- **konachan** has a real adapter boundary and safe-mode browsing surface;
  Cloudflare and bot-detection failures are reported honestly rather than
  treated as something the client should bypass.
- **pixiv** and **twitter** are registered as explicit skeletons so their
  future capabilities have a place in the registry without pretending they
  are ready.
- Full-quality detail media is resolved by Rust and staged through the local
  cache. The browser never talks to a site directly or writes to the download
  library by itself.

The external and authenticated yandere release evidence is still a separate
gate. See [the release gate](docs/YANDE-RELEASE-GATE.md) and
[issue #5](https://github.com/azusachino/dreamland/issues/5) for what remains.

## Why Dreamland

Image-board clients become frustrating when every view is a one-off page and
every download is a fresh network request. Dreamland treats the daily actions
as first-class flows:

1. Pick a site from the top-level site switcher.
2. Browse latest or score-ranked popular content, with pagination preserved
   when switching tabs.
3. Search exact tags with suggestions, or follow an author tag from detail.
4. Open a focused artwork view with the original-quality media when available.
5. Favorite, queue, or download without losing the current browsing context.
6. Return through normal history, reopen the local file, or continue from the
   download history grouped by time.

The product is deliberately a desktop image board, not a generic social feed.
Linux and mobile are outside the current target; there is no distribution
schedule yet.

## Architecture at a glance

```text
┌──────────────────────────────────────────────────────────────┐
│ React 19 + TypeScript + MUI                                │
│ layout · browse · detail · settings · downloads · notices   │
└───────────────────────────────┬──────────────────────────────┘
                                │ typed Tauri commands
┌───────────────────────────────▼──────────────────────────────┐
│ src-tauri                                                   │
│ command handlers · window lifecycle · auth/session bridge   │
└──────────────┬──────────────────────────────┬────────────────┘
               │                              │
┌──────────────▼──────────────┐  ┌────────────▼────────────────┐
│ dreamland-runtime            │  │ dreamland-sites             │
│ config · state · cache       │  │ registry · composition      │
│ downloads · lifecycle        │  │ capability validation       │
└──────────────┬──────────────┘  └───────┬─────────┬────────────┘
               │                         │         │
┌──────────────▼──────────────┐  ┌───────▼───┐ ┌──▼────────────┐
│ dreamland-state              │  │ yandere  │ │ konachan      │
│ SQLite schema + repository   │  │ adapter  │ │ adapter       │
└──────────────┬──────────────┘  └───────────┘ └───────────────┘
               │
┌──────────────▼──────────────────────────────────────────────┐
│ dreamland-core · dreamland-moe                              │
│ site-neutral contracts · shared Moebooru protocol behavior  │
└──────────────────────────────────────────────────────────────┘
```

The contract is intentionally layered:

| Layer | Owns | Must not own |
| --- | --- | --- |
| `dreamland-core` | site-neutral models and object-safe capability ports | HTTP, SQLite, UI details |
| site adapters | endpoints, request mapping, site-specific auth and features | application-wide state or command wiring |
| `dreamland-sites` | adapter registration, capability validation, dispatch | site-specific business rules |
| `dreamland-runtime` | config, cache, downloads, queue/history services | React state or endpoint knowledge |
| `src-tauri` | IPC command composition and native integration | remote protocol implementation |
| React frontend | presentation, navigation, optimistic UI, notifications | secrets, filesystem access, remote requests |

This is the 0.1.1 implementation of the API v1 boundary. Unsupported optional
capabilities are absent from an adapter instead of represented by fake
successes, and the registry validates each descriptor against its actual ports.
Read [API v1](docs/API-V1.md) and
[the SOLID boundary decision](docs/adr/0010-solid-layer-boundaries.md) for the
details.

## Local-first state

Downloads are user-owned. Runtime state is disposable or reconstructible. The
boundaries are visible and documented rather than hidden in a platform-specific
directory:

| Data | Linux default | macOS / Windows | Cleanup |
| --- | --- | --- | --- |
| Settings | `$XDG_CONFIG_HOME/dreamland/config.toml` (`~/.config` fallback) | platform config directory / `dreamland/config.toml` | retained |
| Queue and history | `$XDG_DATA_HOME/dreamland/state.sqlite3` (`~/.local/share` fallback) | platform local-data directory / `dreamland/state.sqlite3` | retained |
| Detail and staging cache | `$XDG_CACHE_HOME/dreamland/{detail,detail-staging,downloads}` | platform cache directory / `dreamland/...` | detail is reusable; staging is disposable |
| Logs | `$XDG_STATE_HOME/dreamland/logs/dreamland.log` (`~/.local/state` fallback) | platform local-data directory / `dreamland/logs/dreamland.log` | retained for diagnostics |
| Download library | configured path; `~/Downloads/dreamland_images` by default | configured path; platform download directory by default | never removed by cache cleanup |

Configuration is TOML. Common settings live at the top level; each site owns a
`[sites.<site-id>]` section for non-secret enablement and versioned extensions.
Credentials and cookies stay in the native session boundary and never enter
TOML or SQLite. A successful download promotes the file into the configured
library and removes its duplicate detail-cache copy. Detail loading falls back
to the library, so clearing cache does not make an already-downloaded image
remote-only. Cache cleanup refuses to run while queued or running image/pool
work exists and removes only cache state.

See the [state contract](docs/STATE.md) for lifecycle rules and the SQLite
schema and repository ownership.

## Technology

| Area | Choice |
| --- | --- |
| Desktop shell | Tauri 2 |
| Frontend | React 19, TypeScript, Vite, Bun |
| UI system | MUI components and theme tokens |
| Async UI state | TanStack Query |
| Runtime | Rust 2024, Tokio, Reqwest |
| Local database | SQLite through `dreamland-state` and `rusqlite` |
| Shared protocol | `dreamland-moe` |
| Tooling | mise, uv, Make |

## Quick start

Use mise for the latest stable Rust, Bun, and uv toolchain. Native Tauri
prerequisites remain OS-level dependencies.

```bash
uv sync --locked
bun install
make doctor
make dev
```

Run the complete local gate:

```bash
make check                  # format, tests, TypeScript, build, tooling
make validate               # release/PR alias for the repository gate
```

Before changing a boundary, read the project-specific guide and the relevant
acceptance stories. The frontend and native runtime are validated separately so
an attractive screen cannot hide a broken command contract.

## Documentation map

- [Development guide](docs/DEVELOPMENT.md) — setup, commands, and troubleshooting
- [agent-browser SOP](docs/AGENT-BROWSER-SOP.md) — isolated browser checks and mandatory resource cleanup
- [Project spec](docs/PROJECT-SPEC.md) — scope and acceptance criteria
- [API v1](docs/API-V1.md) — site/runtime capability contract
- [User stories](docs/USER-STORIES-V1.md) — happy, edge, and failure flows
- [Architecture v1](docs/ARCHITECTURE-V1.md) — ownership, workflow, and dataflow
- [State](docs/STATE.md) — TOML, SQLite, XDG, cache, and logs
- [MoeBooru UX learnings](docs/MOEBOORU-UX-LEARNINGS.md) — reference flows
- [MoeLoader site matrix](docs/MOELOADER-SITE-MATRIX.md) — capability research
- [Yande release gate](docs/YANDE-RELEASE-GATE.md) — external acceptance evidence
- [Decision index](docs/DECISIONS.md) and [ADRs](docs/adr/README.md) — rationale
- [Roadmap](docs/ROADMAP.md) — product direction, not a release schedule
- [Changelog](CHANGELOG.md) — release and engineering notes

## Contributing direction

Keep new behavior behind the documented contracts and user stories. A new
site should implement only the capabilities it genuinely supports, register its
descriptor through the composite, and add contract-level tests. A persistence,
cache, or platform change must update the state contract and its relevant
ADR in the same change.

The project is early enough to fix a bad boundary now. Prefer a small, explicit
interface and a real acceptance test over a compatibility shim that will become
the next release's legacy surface.
