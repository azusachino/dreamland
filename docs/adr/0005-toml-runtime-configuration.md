# ADR 0005: TOML runtime configuration

- Status: Accepted; implementation pending
- Date: 2026-08-10

## Context

Dreamland’s user-owned settings are small and human-edited. They need readable
diffs and manual repair. Tauri’s own build configuration has a separate schema
and format requirement.

## Decision

Use TOML for Dreamland runtime settings. Read TOML first, fall back to the
legacy `config.json` when TOML is absent, validate values in Rust, and write
future changes as TOML. Do not automatically delete the legacy JSON file
during the compatibility window.

Tauri’s `src-tauri/tauri.conf.json` remains JSON because it is consumed by the
Tauri CLI and schema.

## Consequences

- Human-authored runtime settings are readable and repairable.
- Existing users do not lose settings during the migration.
- Favorites, local tags, cache metadata, and download records must use an
  application-data store rather than being forced into TOML.
