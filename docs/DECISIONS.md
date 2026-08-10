# Dreamland decisions

## Branch

`universal` is the active integration branch for the Tauri migration and
cross-platform desktop work. The workstation catalog continues to point at
`main` until `universal` is pushed and becomes the repository's intended
checkout branch.

## Frontend and desktop shell

Dreamland uses React 19, Vite, Bun, and Tauri 2. React owns presentation and
local UI state; Rust owns network requests, configuration persistence, and
downloads. The frontend crosses the boundary through four explicit Tauri
commands instead of receiving filesystem or network capabilities directly.

This keeps the migration small and makes the same web UI usable in Tauri's
desktop WebView without adding a frontend framework beyond the requested React
stack.

## Runtime configuration: JSON

User settings are stored as JSON at the platform config directory under
`dreamland/config.json`.

The desktop command layer uses the `dirs` crate for this location rather than
a Tauri filesystem plugin. That keeps path discovery available to Rust tests
and avoids granting the webview a filesystem scope. If mobile support becomes
a real target, this decision should be revisited in favor of Tauri's
platform-aware app path API.

JSON is the right size and shape for this data: three scalar settings, no
queries, no relationships, no secrets, and no concurrent writers. `serde` and
`serde_json` already provide the serialization path, and a user can inspect or
repair the file without a database tool.

We are not using `tauri-plugin-store` because it would add a plugin and
capability surface without providing value for this small file. We are not
using TOML or YAML because these settings are written by the application, not
maintained as a human-authored project manifest. Tauri's own
`src-tauri/tauri.conf.json` remains JSON independently because that is the
format consumed by the Tauri CLI and schema.

The existing Dioxus-era file shape is retained, and missing
`images_per_page` values default to 20, so this migration does not invalidate
an existing user configuration.

The config file is not a secret store. Future credentials must use the
platform keychain instead of being added to this JSON file.

## Toolchain: mise, not a project Nix flake

`.mise.toml` pins Bun and Rust, including `rustfmt` and `clippy`. This matches
the workstation convention and keeps the project usable on Windows, macOS,
and Linux without requiring Nix.

Tauri still needs native WebView libraries on Linux and the corresponding
WebView toolchain on other platforms. Those are host prerequisites, not
language-version tools, so they are documented rather than hidden behind a
Linux-only flake. A future Linux CI image may use Nix to provision those
libraries, but adding a Nix flake now would not make the universal desktop
build reproducible across all three platforms.

## Security boundary

The webview gets only the core Tauri command capability. Rust validates the API
scheme, rejects empty download paths, checks HTTP status codes, and accepts
only hexadecimal image identifiers for output filenames. Remote previews are
allowed over HTTPS by the CSP; arbitrary frontend filesystem access is not.

## Packaging

Bundling is enabled and uses icons generated from the checked-in
`app-icon.svg`. Signing, notarization, and store-specific configuration are
release concerns and are intentionally not part of this migration.
