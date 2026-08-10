# Changelog

All notable user-visible changes will be recorded here. The project has no
release schedule yet, so this file currently tracks the unreleased foundation
work rather than promising a version date.

## [Unreleased]

### Added

- Tauri 2 desktop foundation for macOS and Windows.
- React 19, TypeScript, Vite, Bun, Tailwind CSS, and TanStack Query frontend.
- Rust workspace boundaries for core contracts, runtime I/O, and the Yandere
  provider adapter.
- Nix development environment and uv-based daily checks.
- API v1 draft, roadmap, reference learnings, and architecture decision records.

### Changed

- Rebuilt the application from the former Dioxus shell into Tauri.
- Moved runtime configuration, provider requests, and downloads behind Rust
  commands.

### Removed

- Mobile and store-specific icon assets from the desktop-only project.
- The project-local mise configuration.
