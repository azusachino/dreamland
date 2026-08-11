# Dreamland

> A native desktop image-board browser and collector for macOS and Windows.

Dreamland is being rebuilt as a focused Tauri application: a React and
TypeScript interface over a Rust runtime that owns site adapters, configuration,
local data, and downloads.

**Status:** 0.1.0 milestone in progress  ·  **Targets:** macOS, Windows  ·
**Site ID:** `yandere`

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

The next architectural gate is approval of API v1. Until then, keep changes
inside the documented boundaries and avoid adding site capabilities, registries,
search, favorites, tags, or batch workflows speculatively. "Speculative" means
implementing or shipping these in code ahead of approval -- the
`docs/API-V1.md`, `docs/ARCHITECTURE-V1.md`, and `docs/USER-STORIES-V1.md`
design-gate documents specifying them ahead of approval is the intended
process, not an exception to it.
