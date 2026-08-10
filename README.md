# Dreamland

> A native desktop image-board browser and collector for macOS and Windows.

Dreamland is being rebuilt as a focused Tauri application: a React and
TypeScript interface over a Rust runtime that owns providers, configuration,
local data, and downloads.

**Status:** foundation rework in progress  ·  **Targets:** macOS, Windows  ·
**Provider ID:** `yandere`

## What exists now

- Tauri 2 desktop shell
- React 19 + TypeScript + Vite + Bun frontend
- Typed Tauri IPC wrappers
- Tailwind CSS styling foundation
- TanStack Query for command-backed asynchronous state
- Rust Cargo workspace with core, runtime, and Yandere adapter boundaries
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
        ├── dreamland-core          provider-neutral domain contracts
        └── dreamland-provider-yandere
                                    yande.re adapter and fixtures
```

The frontend does not call providers or access the filesystem directly.
Provider traits, registry behavior, and the provider-neutral v1 command model
remain gated by [API and runtime design v1](docs/API-V1.md).

## Stack

| Layer | Choice | Role |
| --- | --- | --- |
| Desktop | Tauri 2 | Native macOS/Windows shell and IPC |
| Frontend | React 19, TypeScript, Vite, Bun | UI and local interaction state |
| Async UI state | TanStack Query | Cache and lifecycle for Rust commands |
| Styling | Tailwind CSS v4 | Utility styling through Vite |
| Runtime | Rust | Providers, validation, persistence, and downloads |
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
- [API v1 draft](docs/API-V1.md) — contract gate before provider/runtime traits
- [Architecture decisions](docs/DECISIONS.md) — decision index
- [ADRs](docs/adr/README.md) — rationale and consequences of accepted decisions
- [Roadmap](docs/ROADMAP.md) — product direction, not a release schedule
- [Reference learnings](docs/REFERENCE-LEARNINGS.md) — yande and MoeLoaderP notes
- [Changelog](CHANGELOG.md) — unreleased and future user-visible changes

## Contributing direction

The next architectural gate is approval of API v1. Until then, keep changes
inside the documented boundaries and avoid adding provider traits, registries,
search, favorites, tags, or batch workflows speculatively.
