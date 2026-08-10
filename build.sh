#!/usr/bin/env bash
set -euo pipefail

command -v bun >/dev/null || { echo "Bun is required" >&2; exit 1; }
command -v cargo >/dev/null || { echo "Rust/Cargo is required" >&2; exit 1; }

bun install --frozen-lockfile
bun run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
