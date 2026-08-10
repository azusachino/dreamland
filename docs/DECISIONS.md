# Dreamland decisions

This document records the current direction. The decisions marked as
“planned” are intentionally not implemented in this stop point.

## Product and platform scope

macOS and Windows are Dreamland’s first-class desktop platforms. Development
will continue from a macOS machine, with Windows validation performed on a
native Windows environment or CI runner.

Linux is not a planned target yet. We will not add Linux-specific components,
Linux packaging, Linux CI requirements, or Linux troubleshooting as part of
this migration. The code may still compile elsewhere incidentally, but Linux
is outside the supported acceptance matrix until that decision changes.

## Branching

The migration uses normal descriptive feature branches. The Dreamland branch
is `feat/migrate-to-tauri`; the workstation branch tracking this vendored
change is `feat/vendor-dreamland-tauri`. There is no special `universal`
branch convention.

## Frontend and desktop shell

Dreamland uses React 19, Vite, Bun, and Tauri 2. React owns presentation and
local UI state; Rust owns network requests, configuration persistence, and
downloads. The frontend crosses the boundary through four explicit Tauri
commands instead of receiving filesystem or network capabilities directly.

This keeps the migration small and makes the same web UI usable in Tauri’s
desktop WebView without adding another frontend framework.

## Project toolchain: Nix only (planned)

The project-level toolchain decision is Nix. `mise` is not the long-term
project environment and should be removed when the toolchain migration is
implemented. The Nix definition should pin Rust, Bun, and the command-line
tools needed by the project.

There is one platform constraint: Nix is not a native Windows provisioning
layer. The feasible counterpart for first-class Windows support is to keep Nix
as the source of truth for shared versions and macOS development, while using
native Windows runners for Windows builds and platform SDK requirements. The
Windows setup must consume the same pinned Rust and Bun versions; it must not
introduce a second project-specific version policy.

No Nix flake is added in this decision-only pass.

## Runtime configuration: TOML (planned)

Dreamland’s user-owned runtime settings will move to TOML in the platform
configuration directory. TOML is chosen because this is a human-editable
settings file: comments, readable diffs, and straightforward manual repair
are useful, while the configuration remains small and has no query or
relationship requirements.

This applies only to Dreamland’s runtime settings. Tauri’s own
`src-tauri/tauri.conf.json` remains JSON because that is the format consumed by
the Tauri CLI and its schema.

The migration should remain backwards-compatible: read the new TOML file
first, fall back to the existing `config.json` when TOML is absent, validate
the values, and write subsequent changes as TOML. The legacy JSON file should
not be deleted automatically during the first migration release.

The current checkout still reads and writes JSON. The TOML migration, including
its compatibility tests, is future implementation work.

## Security boundary

The WebView gets only the core Tauri command capability. Rust validates the API
scheme, rejects empty download paths, checks HTTP status codes, and accepts
only hexadecimal image identifiers for output filenames. Remote previews are
allowed over HTTPS by the CSP; arbitrary frontend filesystem access is not.

## Packaging changes (planned)

The Tauri bundle remains the packaging boundary, with icons generated from the
checked-in `app-icon.svg`. Packaging will be defined for the two first-class
platforms:

- macOS: an `.app` bundle, with a `.dmg` distribution artifact when release
  distribution needs it;
- Windows: a native installer artifact, initially `.msi` unless release
  testing shows that an `.exe` installer is a better fit.

Linux packages and Linux-specific packaging dependencies are out of scope.
macOS signing/notarization and Windows signing, installer UX, and distribution
channels remain separate release decisions. No packaging configuration changes
are made in this decision-only pass.

## Explicitly deferred

The following are recorded decisions, not completed changes:

- add the Nix project environment and remove `.mise.toml`;
- migrate runtime settings from JSON to TOML with legacy JSON fallback;
- update Tauri packaging configuration for native macOS and Windows artifacts;
- define the macOS and Windows build/release matrix;
- decide signing, notarization, and installer distribution details for each
  supported platform.
