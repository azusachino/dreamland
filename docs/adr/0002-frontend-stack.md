# ADR 0002: Frontend stack

- Status: Accepted
- Date: 2026-08-10

## Context

The frontend will grow from a small gallery into a site-aware desktop
collector. It needs typed IPC, predictable asynchronous state, and a styling
system that can scale without adding a framework server or mobile target.

## Decision

Use React 19, strict TypeScript, Vite, and Bun. Use Tailwind CSS v4 through
its Vite plugin for styling and TanStack Query for asynchronous Tauri-command
state. Keep ordinary React state for local interaction state.

Do not add a router, global store, or component library while Dreamland has one
window and one primary view. Add those only when the product has a concrete
navigation or shared-state problem that justifies them.

## Consequences

- IPC payloads are represented by typed wrappers in `src/lib/ipc.ts`.
- TypeScript checking is a daily build gate.
- TanStack Query can manage caching, invalidation, pagination, and mutations
  without moving network access into the WebView.
- The frontend avoids speculative state-management and UI dependencies.
