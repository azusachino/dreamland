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

Nix is the only project-level toolchain source of truth. The macOS flake pins
Rust, Bun, Make, native build inputs, and uv. uv plus small Python scripts is
the daily tooling layer. Windows uses native runners and the same pinned
Rust, Bun, and uv versions because Nix is not a native Windows provisioning
layer.

The desktop bundle keeps the five generated icon assets required by the
current Tauri configuration: `32x32.png`, `128x128.png`, `128x128@2x.png`,
`icon.icns`, and `icon.ico`. Mobile and store-specific icon assets are not
part of the product scope.

## Consequences

- The flake has Darwin outputs only.
- Windows validation must happen on a native Windows environment or runner.
- `pyproject.toml` and `uv.lock` describe daily scripts, not an alternative
  to Nix for the project toolchain.
- Linux hosts may run incidental tooling, but do not define support work.
