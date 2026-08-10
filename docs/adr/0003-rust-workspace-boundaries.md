# ADR 0003: Rust workspace boundaries

- Status: Accepted
- Date: 2026-08-10

## Context

The Tauri crate should remain a desktop shell, while provider contracts,
provider adapters, and runtime-owned I/O need independent tests and compile
boundaries. A single crate would make the command layer own too much.

## Decision

Use a virtual Cargo workspace with four members:

- `dreamland-core` — provider-neutral identifiers and draft domain metadata;
- `dreamland-runtime` — configuration, persistence, download orchestration,
  and runtime-owned I/O;
- `dreamland-provider-yandere` — the Yandere adapter and response fixtures;
- `src-tauri` — Tauri commands, window lifecycle, and desktop capabilities.

The workspace owns the single `Cargo.lock` and shared build output. Provider
traits and registry behavior are not implemented until API v1 is approved.

## Consequences

- Pure runtime/provider tests do not require the Tauri shell.
- The Tauri crate remains a narrow command boundary.
- New providers can receive their own crate when their adapter is substantial;
  we do not create speculative crates for providers not yet implemented.
