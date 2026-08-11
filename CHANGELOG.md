# Changelog

All notable user-visible changes will be recorded here. The current unreleased
entry tracks the 0.1.0 milestone; the release gate remains authoritative for
whether that milestone is ready to ship.

## [Unreleased]

### Added

- Tauri 2 desktop foundation for macOS and Windows.
- React 19, TypeScript, Vite, Bun, Tailwind CSS, and TanStack Query frontend.
- Rust workspace boundaries for core contracts, runtime I/O, and the Yandere
  site adapter.
- Nix development environment and uv-based daily checks.
- API v1 draft, roadmap, reference learnings, and architecture decision records.

### Changed

- Rebuilt the application from the former Dioxus shell into Tauri.
- Moved runtime configuration, site requests, and downloads behind Rust
  commands.
- Shared infinite scrolling now rebinds to the active browse query when
  switching between latest, popular, and tag search.
- Removed the unused Reqwest JSON feature and empty `tauri-build` feature
  declaration from the Rust workspace.

### Removed

- Mobile and store-specific icon assets from the desktop-only project.
- The project-local mise configuration.
