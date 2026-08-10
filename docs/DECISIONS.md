# Dreamland decisions

This document records the current direction. The decisions marked as
“planned” are intentionally deferred follow-up work.

## Product and platform scope

macOS and Windows are Dreamland’s first-class desktop platforms. Development
will continue from a macOS machine, with Windows validation performed on a
native Windows environment or CI runner.

Linux is not a planned target yet. We will not add Linux-specific components,
Linux packaging, Linux CI requirements, or Linux troubleshooting as part of
this rework. The code may still compile elsewhere incidentally, but Linux
is outside the supported acceptance matrix until that decision changes.

## Branching

The rework uses normal descriptive feature branches. The Dreamland branch
is `feat/migrate-to-tauri`; the workstation branch tracking this vendored
change is `feat/vendor-dreamland-tauri`. There is no special cross-platform
branch convention.

## Frontend and desktop shell

Dreamland uses React 19, Vite, Bun, and Tauri 2. React owns presentation and
local UI state; Rust owns network requests, configuration persistence, and
downloads. The frontend crosses the boundary through four explicit Tauri
commands instead of receiving filesystem or network capabilities directly.

The frontend source is strict TypeScript. Typed IPC wrappers keep Tauri
command payloads out of individual components. Tailwind CSS v4 is the styling
layer through its Vite plugin. TanStack Query owns asynchronous Rust-command
state; React state remains for local interaction state. No router, global store,
or component library is required while the desktop app has one window and one
primary view.

This keeps the rework small and makes the same web UI usable in Tauri’s
desktop WebView without adding another frontend framework.

## Rust workspace

The Rust side is a Cargo workspace with four members: `dreamland-core` for
provider-neutral identifiers and descriptors, `dreamland-runtime` for config
and download I/O, `dreamland-provider-yandere` for the Yandere adapter, and
`src-tauri` for the desktop shell and command registration. The Tauri crate is
kept as the application boundary; it does not own provider HTTP or runtime
filesystem logic.

Provider traits and the registry remain gated by [API and runtime design
v1](API-V1.md). The workspace layout makes that boundary possible without
pretending the draft contract is already final.

## API and runtime contract

The existing API and runtime ownership model is part of the product contract.
Rust remains responsible for provider requests, configuration persistence,
validation, and downloads. React communicates through narrow provider-neutral
Tauri commands; it does not call providers or access the filesystem directly.

Dreamland supports a set of providers. `yande.re` is one complete provider,
not the application-wide API assumption. Provider-specific capabilities such
as search and pagination must be represented at the provider boundary.

## Project toolchain: Nix only

The project-level toolchain decision is Nix. The Nix definition should pin
Rust, Bun, and the command-line tools needed by the project. No competing
project-level toolchain file is kept in the repository.

`flake.nix` provides a development shell for `aarch64-darwin` and
`x86_64-darwin`, including Rust 1.97.1 with rustfmt and clippy, Bun, Make,
OpenSSL, pkg-config, and uv. Enter it with `nix develop` before running the
Make targets or Bun commands. The `pyproject.toml` and `uv.lock` files only
describe the small daily-tooling environment; they do not replace Nix as the
project toolchain decision.

There is one platform constraint: Nix is not a native Windows provisioning
layer. The feasible counterpart for first-class Windows support is to keep Nix
as the source of truth for shared versions and macOS development, while using
native Windows runners for Windows builds and platform SDK requirements. The
Windows setup must consume the same pinned Rust and Bun versions; it must not
introduce a second project-specific version policy.

Linux is intentionally absent from the flake outputs. Windows remains a
first-class native build target, but uses a native Windows runner for platform
SDKs and the same pinned Rust and Bun versions because Nix is not a native
Windows provisioning layer.

## Runtime configuration: TOML (planned)

Dreamland’s user-owned runtime settings will move to TOML in the platform
configuration directory. TOML is chosen because this is a human-editable
settings file: comments, readable diffs, and straightforward manual repair
are useful, while the configuration remains small and has no query or
relationship requirements.

This applies only to Dreamland’s runtime settings. Tauri’s own
`src-tauri/tauri.conf.json` remains JSON because that is the format consumed by
the Tauri CLI and its schema.

The configuration change should remain backwards-compatible: read the new TOML file
first, fall back to the existing `config.json` when TOML is absent, validate
the values, and write subsequent changes as TOML. The legacy JSON file should
not be deleted automatically during the first release using TOML.

The current checkout still reads and writes JSON. TOML configuration, including
its compatibility tests, is future implementation work.

## Security boundary

The WebView gets only the core Tauri command capability. Rust validates the API
scheme, rejects empty download paths, checks HTTP status codes, and accepts
only hexadecimal image identifiers for output filenames. Remote previews are
allowed over HTTPS by the CSP; arbitrary frontend filesystem access is not.

## Release planning

There is no release plan yet. Signing, notarization, installers, distribution
channels, automatic updates, and store submission are deliberately not
roadmap commitments. The current Tauri bundle configuration only supports
development of the desktop application.

## Explicitly deferred

The following are recorded decisions, not completed changes:

- adopt TOML runtime settings with a legacy JSON fallback;
- define application-data storage for favorites and tags;
- implement the provider contract and registry;
- add provider-aware search, tags, favorites, and batch downloads.
