# Development Guide

## Quick Start

```bash
nix develop
uv sync --locked
bun install
bun run tauri:dev
```

The frontend runs on Vite during Tauri development. Rust commands live under
`src-tauri/` and handle API requests, configuration, and downloads. Daily
tooling and platform-aware checks live in `scripts/` and run through uv.

Or use `make install`, `make doctor`, `make dev`, and `make check` inside
`nix develop`.

## Commands

- `bun run dev` — start the Vite frontend only
- `bun run build` — build the React frontend into `dist/`
- `bun run tauri:dev` — run the desktop application
- `bun run tauri:build` — build the Tauri application
- `bun run typecheck` — check TypeScript
- `uv run scripts/doctor.py` — check the local toolchain
- `uv run scripts/check.py` — run the daily platform-aware checks
- `cargo fmt --all -- --check` — check Rust formatting
- `cargo test --workspace` — run Rust tests

## Architecture

```text
React + TypeScript (src/) ── typed Tauri IPC ── commands (src-tauri/src/lib.rs)
                                                   ├── site registry/composition
                                                   ├── runtime crate
                                                   └── site-neutral core ports
```

The frontend calls `load_config`, `save_config`, `query_posts`,
`continue_query`, `cancel_query`, `suggest_tags`, `enqueue_download`,
`cancel_download`, `retry_download`, and `list_downloads`. Native filesystem,
SQLite, and network access stays in Rust. Network settings are applied on the
next operation without restarting the app. Site enablement is loaded from the
TOML `[sites.<site-id>]` sections when the runtime builds its registry; the
composition crate is the only place that registers concrete adapters.

Download queue state is stored in the runtime-owned SQLite database under the
platform data directory. Temporary `.part` files are written under the
runtime-owned cache; completed files are committed below the configured
download root and existing targets terminate as `ExistingTarget`.

Architecture rationale is recorded in [DECISIONS.md](DECISIONS.md) and
[adr/](adr/). The API boundary is still governed by [API-V1.md](API-V1.md).

## Troubleshooting

- **macOS setup:** enter `nix develop` and install the native macOS Tauri
  prerequisites before running the desktop build.
- **Windows setup:** use a native Windows environment or CI runner with the
  pinned Rust and Bun versions and the native Tauri build prerequisites.
- **Network errors:** check the active site's bundled endpoint, use Settings →
  Detect proxy, and choose Direct or Manual proxy if Auto is wrong. GET requests
  retry bounded 429/temporary responses and honor numeric `Retry-After` hints.
- **Permission errors:** ensure the configured download directory is writable.
- **Configuration reset:** remove `config.toml` from the platform configuration
  directory. Runtime settings are TOML-only; the cache action does not remove
  configuration.
