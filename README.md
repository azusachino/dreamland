# Dreamland Image Viewer

A cross-platform image gallery application built with Tauri, React, Vite, and
Rust. It supports image-board APIs using the `yande.re/post.json` format.

macOS and Windows are the first-class desktop targets. Linux is not currently
supported and is not part of the rework acceptance matrix.

## Features

- Browse images in a responsive gallery
- Fetch image metadata from a configurable API
- Download high-quality images to a configurable local directory
- Paginate through image results
- Save settings in the platform user configuration directory

## Stack

- Tauri 2
- React 19
- Vite
- Bun
- Rust
- Nix

## Development

Install dependencies and start the Tauri development window:

```bash
nix develop
uv sync --locked
bun install
bun run tauri:dev
```

The same commands are available through the repository Makefile:

```bash
make install
make dev
make check
```

The Nix flake provides the pinned Rust, Bun, Make, OpenSSL, and pkg-config
toolchain plus uv for macOS. Windows uses native runners with the same Rust,
Bun, and uv tool versions because Nix is not a native Windows provisioning
layer. Use `uv run scripts/doctor.py` to check the local toolchain and
`uv run scripts/check.py` for the daily platform-aware checks.
Run the frontend build:

```bash
bun run build
```

Run Rust checks:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
```

## Project Structure

```text
src/                    # React frontend
src-tauri/              # Tauri application and Rust commands
  src/api.rs            # API client and image metadata
  src/config.rs         # Persistent settings
  src/lib.rs            # Tauri command registration
index.html              # Vite entry document
vite.config.js          # Vite configuration
docs/DECISIONS.md       # Architecture and tooling decisions
```

The frontend calls four narrow Tauri commands: `load_config`, `save_config`,
`load_images`, and `download_image`. Filesystem and network access remain in
Rust rather than being exposed directly to the webview.

See [docs/DECISIONS.md](docs/DECISIONS.md) for the platform scope, TOML
configuration, security boundary, and tooling decisions.

## Configuration

The current implementation stores settings at:

- The platform configuration directory on macOS
- The platform config directory on Windows

The planned format is TOML. The current JSON format remains until that
configuration change is implemented.

Default settings:

```json
{
  "download_path": "~/Downloads/dreamland_images",
  "api_url": "https://yande.re/post.json",
  "images_per_page": 20
}
```

## API Format

The configured API returns an array of objects with these fields:

```json
[
  {
    "id": 123456,
    "tags": "tag1 tag2 tag3",
    "width": 1920,
    "height": 1080,
    "file_url": "https://example.com/full_image.jpg",
    "sample_url": "https://example.com/sample.jpg",
    "preview_url": "https://example.com/preview.jpg",
    "rating": "s",
    "score": 42,
    "md5": "abcdef123456",
    "file_size": 1048576
  }
]
```
