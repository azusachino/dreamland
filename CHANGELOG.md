# Changelog

All notable user-visible changes will be recorded here. The release gate
remains authoritative for external and authenticated acceptance.

## [Unreleased]

No unreleased changes.

## [0.1.1] - 2026-08-12

This is an engineering release candidate. It is versioned and checked locally;
distribution and the external Yande release gate remain separate decisions.

### Added

- TOML-only runtime configuration with explicit per-site sections.
- Dedicated `dreamland-local-state` SQLite schema and repository-infrastructure
  crate with versioned migrations.
- SOLID layer audit and registry follow-up decision for the core/site/runtime
  boundary.

### Changed

- Documented the MVC-style controller, application-service, repository, model,
  and infrastructure ownership split.
- Corrected the API and architecture documents to describe only the SQLite
  tables and cache paths implemented in 0.1.1.
- Added explicit local-cache cleanup behavior and XDG/platform state
  documentation.

### Foundation included

- Tauri 2 desktop foundation for macOS and Windows.
- React 19, TypeScript, Vite, Bun, Tailwind CSS, and TanStack Query frontend.
- Rust workspace boundaries for core contracts, runtime I/O, and the Yandere
  site adapter.
- Nix development environment and uv-based daily checks.
- API v1 baseline, roadmap, reference learnings, and architecture decision
  records.
- Shared infinite scrolling now rebinds to the active browse query when
  switching between latest, popular, and tag search.
- Removed the unused Reqwest JSON feature and empty `tauri-build` feature
  declaration from the Rust workspace.

### Removed from the foundation

- Mobile and store-specific icon assets from the desktop-only project.
- The project-local mise configuration.
