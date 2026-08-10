# Dreamland Image Viewer

A cross-platform image gallery application built with Tauri, React, Vite, and
Rust. It supports image-board APIs using the `yande.re/post.json` format.

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

## Development

Install dependencies and start the Tauri development window:

```bash
bun install
bun run tauri:dev
```
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
```

The frontend calls four narrow Tauri commands: `load_config`, `save_config`,
`load_images`, and `download_image`. Filesystem and network access remain in
Rust rather than being exposed directly to the webview.

## Configuration

Settings are stored at:

- `~/.config/dreamland/config.json` on Linux/macOS
- The platform config directory on Windows

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
