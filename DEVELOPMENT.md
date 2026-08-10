# Development Guide

## Quick Start

```bash
bun install
bun run tauri:dev
```

The frontend runs on Vite during Tauri development. Rust commands live under
`src-tauri/` and handle API requests, configuration, and downloads.

Or use `make install`, `make dev`, and `make check`. Run `mise install` first
to use the pinned Bun and Rust versions.

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

- **Linux native build errors:** install Tauri's WebKitGTK 4.1 and librsvg
  prerequisites for your distribution.
- **Network errors:** check the configured API URL and connectivity.
- **Permission errors:** ensure the configured download directory is writable.
- **Configuration reset:** delete `~/.config/dreamland/config.json` on Linux/macOS.
