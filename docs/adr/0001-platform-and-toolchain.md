# ADR 0001: Platform and toolchain

- Status: Accepted
- Date: 2026-08-10

## Context

Dreamland needs a reproducible macOS development environment and a credible
Windows counterpart. Linux is not a planned product target. The project also
needs small daily tooling and test orchestration without creating a competing
project toolchain policy.

## Decision

macOS and Windows are first-class desktop targets. Linux-specific components,
packaging, CI requirements, and support commitments are out of scope.

mise is the project-level toolchain source of truth. `.mise.toml` selects the
latest stable Rust, Bun, and uv toolchain; native build inputs remain
OS-level prerequisites. uv plus small Python scripts is the daily tooling
layer, and both desktop platforms use the same runtime selection.

The desktop bundle keeps the five generated icon assets required by the
current Tauri configuration: `32x32.png`, `128x128.png`, `128x128@2x.png`,
`icon.icns`, and `icon.ico`. Mobile and store-specific icon assets are not
part of the product scope.

## Consequences

- Windows validation must happen on a native Windows environment or runner.
- `pyproject.toml` and `uv.lock` describe the daily scripts and their
  dependencies; `.mise.toml` selects the runtimes used to run them.
- Linux hosts may run incidental tooling, but do not define support work.
