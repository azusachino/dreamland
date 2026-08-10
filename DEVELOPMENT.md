# Development Guide

## Quick Start

```bash
nix develop
bun install
bun run tauri:dev
```

The frontend runs on Vite during Tauri development. Rust commands live under
`src-tauri/` and handle API requests, configuration, and downloads.

Or use `make install`, `make dev`, and `make check` inside `nix develop`.

## Commands

- `bun run dev` — start the Vite frontend only
- `bun run build` — build the React frontend into `dist/`
- `bun run tauri:dev` — run the desktop application
- `bun run tauri:build` — build the Tauri application
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` — check Rust formatting
- `cargo test --manifest-path src-tauri/Cargo.toml` — run Rust tests

## Architecture

```text
React + Vite (src/) ── Tauri IPC ── Rust commands (src-tauri/src/lib.rs)
                                      ├── API client (api.rs)
                                      ├── config persistence (config.rs)
                                      └── image downloads
```

The frontend calls `load_config`, `save_config`, `load_images`, and
`download_image`. Native filesystem and network access stays in Rust.

## Troubleshooting

- **macOS setup:** enter `nix develop` and install the native macOS Tauri
  prerequisites before running the desktop build.
- **Windows setup:** use a native Windows environment or CI runner with the
  pinned Rust and Bun versions and the native Tauri build prerequisites.
- **Network errors:** check the configured API URL and connectivity.
- **Permission errors:** ensure the configured download directory is writable.
- **Configuration reset:** remove `config.json` from the platform configuration
  directory. The current implementation still uses JSON; TOML configuration
  is planned.
