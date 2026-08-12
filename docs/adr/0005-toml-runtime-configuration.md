# ADR 0005: TOML runtime configuration

- Status: Accepted; implemented
- Date: 2026-08-10

## Context

Dreamland’s user-owned settings are small and human-edited. They need readable
diffs and manual repair. Tauri’s own build configuration has a separate schema
and format requirement.

## Decision

Use TOML for Dreamland runtime settings. Read `config.toml`, validate values in
Rust, and write future changes as TOML. This is a fresh 0.1.0 project, so there
is no JSON compatibility format.

Common settings are top-level fields with a `[network]` section. Each site has
a `[sites.<site-id>]` section containing only non-secret enablement and a
versioned extension map. Site adapter crates continue to own bundled endpoint
and browser defaults; those URLs are not user-editable application settings.

Tauri’s `src-tauri/tauri.conf.json` remains JSON because it is consumed by the
Tauri CLI and schema.

## Consequences

- Human-authored runtime settings are readable and repairable.
- A fresh install has one unambiguous settings format.
- Site-specific configuration has an explicit section without leaking site
  protocol details into the common runtime model.
- Favorites, local tags, cache metadata, and download records must use an
  application-data store rather than being forced into TOML.
