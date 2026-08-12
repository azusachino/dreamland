# Dreamland

> A native desktop image-board browser and collector for macOS and Windows.

[![status: 0.1.0 foundation merged](https://img.shields.io/badge/status-0.1.0%20foundation%20merged-5b7f86)](https://github.com/azusachino/dreamland/issues/5)
[![targets: macOS and Windows](https://img.shields.io/badge/targets-macOS%20%7C%20Windows-5b7f86)](docs/adr/0001-platform-and-toolchain.md)
[![Tauri 2](https://img.shields.io/badge/Tauri-2-24c8db)](https://v2.tauri.app/)
[![React 19](https://img.shields.io/badge/React-19-61dafb)](https://react.dev/)

Dreamland is being rebuilt as a focused Tauri application: a React and
TypeScript interface over a Rust runtime that owns site adapters, configuration,
local data, and downloads.

**Status:** 0.1.0 foundation merged  ·  **Targets:** macOS, Windows  ·
**Site ID:** `yandere`  ·  **Next blockers:** [0.1.1 issue #5](https://github.com/azusachino/dreamland/issues/5)

## What exists now

- Tauri 2 desktop shell
- React 19 + TypeScript + Vite + Bun frontend
- Typed Tauri IPC wrappers
- Tailwind CSS styling foundation
- TanStack Query for command-backed asynchronous state
- Rust Cargo workspace with site-neutral core, shared Moebooru protocol, runtime,
  and site adapter boundaries
- Browse, pagination, settings, and image downloads through Rust commands

Linux and mobile are not planned targets. There is no release schedule or
distribution plan yet.

## Local state and paths

Dreamland separates user-owned downloads from disposable runtime files. On
Linux, the runtime honors the XDG variables shown below; macOS and Windows use
the platform directories resolved by the `dirs` crate. The exact paths are
created only when the corresponding feature needs them.

| Data | Linux default | macOS / Windows | Cleanup policy |
| --- | --- | --- | --- |
| Settings | `$XDG_CONFIG_HOME/dreamland/config.json` (`~/.config` fallback) | platform config directory / `dreamland/config.json` | kept by cache cleanup |
| Queue and history | `$XDG_DATA_HOME/dreamland/state.sqlite3` (`~/.local/share` fallback) | platform local-data directory / `dreamland/state.sqlite3` | kept by cache cleanup |
| Download staging | `$XDG_CACHE_HOME/dreamland/downloads` (`~/.cache` fallback) | platform cache directory / `dreamland/downloads` | removed by **Settings → clear local cache** |
| Detail-image cache | `$XDG_CACHE_HOME/dreamland/detail` | platform cache directory / `dreamland/detail` | removed by **Settings → clear local cache** |
| Detail staging | `$XDG_CACHE_HOME/dreamland/detail-staging` | platform cache directory / `dreamland/detail-staging` | removed by **Settings → clear local cache** |
| Logs | `$XDG_STATE_HOME/dreamland/logs/dreamland.log` (`~/.local/state` fallback) | platform local-data directory / `dreamland/logs/dreamland.log` | kept for diagnostics |
| Download library | configured path; default `~/Downloads/dreamland_images` | configured path; platform download directory by default | never removed by cache cleanup |

The cache action is deliberately narrow: it refuses while a download or pool
archive is running, then removes only temporary staging and detail previews.
It does not reset settings, the SQLite queue/history, the configured download
library, or the site login WebView session.

## Architecture

```text
React + TypeScript
        │ typed Tauri IPC
        ▼
src-tauri/                  desktop commands and window lifecycle
        │
        ├── dreamland-runtime       config, persistence, downloads
        ├── dreamland-core          site-neutral domain contracts
        ├── dreamland-moe           shared Moebooru wire/protocol behavior
        ├── dreamland-sites          composition root and site registry
        ├── dreamland-site-yandere   active Yande adapter and fixtures
        ├── dreamland-site-konachan safe-mode Konachan adapter and fixtures
        ├── dreamland-site-pixiv     descriptor-only skeleton
        └── dreamland-site-twitter   descriptor-only skeleton
```

The frontend does not call remote sites or access the filesystem directly.
Site capabilities, registry behavior, and the site-neutral v1 command model
follow [API and runtime design v1](docs/API-V1.md); external and authenticated
release evidence remains tracked by the release gate.

## Stack

| Layer | Choice | Role |
| --- | --- | --- |
| Desktop | Tauri 2 | Native macOS/Windows shell and IPC |
| Frontend | React 19, TypeScript, Vite, Bun | UI and local interaction state |
| Async UI state | TanStack Query | Cache and lifecycle for Rust commands |
| Styling | Tailwind CSS v4 | Utility styling through Vite |
| Runtime | Rust | Site adapters, validation, persistence, and downloads |
| Toolchain | Nix + uv scripts | Reproducible development and daily checks |

## Quick start

On macOS, enter the Nix development shell. On Windows, use the native Rust,
Bun, and uv toolchain with the same pinned versions; Nix is not used as a
native Windows provisioning layer.

```bash
nix develop                 # macOS
uv sync --locked
bun install
make doctor
make dev
```

Run the daily checks:

```bash
make check
```

The check command validates the uv lockfile, Python tooling, TypeScript,
frontend build, and Rust formatting. Native Tauri tests run on macOS and
Windows; Linux is intentionally outside the acceptance matrix.

## Documentation

- [Development guide](docs/DEVELOPMENT.md) — setup, commands, and troubleshooting
- [Project spec](docs/PROJECT-SPEC.md) — scope and acceptance criteria
- [API v1](docs/API-V1.md) — implementation baseline for site/runtime capabilities
- [v1 user stories](docs/USER-STORIES-V1.md) — user-facing acceptance and
  failure/edge-case coverage
- [v1 architecture](docs/ARCHITECTURE-V1.md) — ownership, workflows, dataflow,
  and security boundaries
- [MoeBooru UX learnings](docs/MOEBOORU-UX-LEARNINGS.md) — Android reference
  flows for exploration, query autocomplete, popular views, downloads, and
  account actions
- [MoeLoaderP site matrix](docs/MOELOADER-SITE-MATRIX.md) — pinned site,
  capability, config, and auth research
- [Yande release gate](docs/YANDE-RELEASE-GATE.md) — feature scope and
  verification evidence required before release
- [Architecture decisions](docs/DECISIONS.md) — decision index
- [ADRs](docs/adr/README.md) — rationale and consequences of accepted decisions
- [Roadmap](docs/ROADMAP.md) — product direction, not a release schedule
- [Reference learnings](docs/REFERENCE-LEARNINGS.md) — yande and MoeLoaderP notes
- [Changelog](CHANGELOG.md) — unreleased and future user-visible changes

## Contributing direction

API v1 and the site/runtime boundaries are the implementation baseline. Keep
new site capabilities behind the documented contracts, release gates, and
user stories; record a cross-platform path or persistence change here and in
the relevant runtime documentation when it changes ownership or cleanup.
