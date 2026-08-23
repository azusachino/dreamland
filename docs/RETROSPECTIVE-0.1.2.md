# Dreamland 0.1.2 retrospective

This is the working record of what the 0.1.2 effort taught us. It is about
boundaries, failure modes, and repeatable checks—not a changelog of every UI
adjustment. Read it before changing navigation, downloads, cache policy, or
the project toolchain.

## The central lesson

Dreamland has two kinds of state that can look identical to the user but have
different owners:

```text
React Query route cache       → what can still be rendered
Rust runtime cache            → what IPC commands can immediately trust
SQLite + library files        → what survives a restart
```

Keeping one layer bounded or disposable is good. Assuming the other layers
have the same lifetime is the bug. A route may render an old post from React
Query after the Rust post cache has been replaced. Commands that need
authoritative metadata must therefore rehydrate on a cache miss instead of
trusting that a visible card is already present in native state.

## What we learned

### 1. A visible UI object is not proof of backend readiness

The download regression appeared after navigating away from a tag search and
returning through browser history. React Query reused the old page, while the
Rust query path had cleared its bounded post map for the intervening query.
The card was visible, but `enqueue_download` could not resolve its post.

Rule: every IPC command must define its cache-miss behavior. For downloads,
the safe behavior is one site-owned lookup by `(site, post_id)`, followed by
cache restoration and URL resolution through the adapter. Do not make the
frontend send a trusted media URL.

Regression shape:

```text
query A → query B replaces native post cache → route cache restores A
       → download A must rehydrate A and succeed
```

### 2. Cache, library, and staging are different products

- The configured download library is durable user media.
- The detail cache is reusable but replaceable presentation state.
- `detail-staging` and temporary download files are disposable in-flight state.
- SQLite queue/history is durable operational history, not the media itself.

Detail loading now prefers the library, falls back to the detail cache, and
refreshes the detail cache once after a browser decode failure. Refresh never
deletes the user library. Download history thumbnails use the local library
first and the remote preview second.

Rule: cleanup and recovery must name the exact layer they mutate. “Clear
cache” must never mean “delete downloads.”

### 3. Download history records attempts, not unique files

`Completed` means a new file was written. `ExistingTarget` means a later
attempt found that canonical file and intentionally did not overwrite it. Both
records are useful evidence, so silently merging them would lose operational
history.

The UI therefore uses short, distinct vocabulary:

| State | User meaning |
| --- | --- |
| `downloaded` | a new file was written |
| `on disk` | the target already existed; no overwrite happened |

Summary counts use unique target paths, while history continues to show
attempts. A row should not repeat the same explanation in its badge and note.

### 4. Progress must represent knowledge, not animation

An unknown total is not zero progress. Queued work is waiting, a running
unknown-size transfer is indeterminate, and a known-size transfer can expose
bytes and percentage. Overall progress aggregates only known totals and names
unknown work explicitly.

Rule: a progress bar must have a terminal state and a truthful source of
movement. If bytes do not change, it must not look like a successful transfer
by looping forever.

### 5. Navigation is an asynchronous lifecycle boundary

Changing tabs does more than change a heading. It can start or cancel a query,
reuse a cached page, remount detail UI, change the active site, and alter which
controls are enabled. Back/forward navigation is therefore a first-class
acceptance path, not just a router smoke test.

Minimum route matrix for future changes:

```text
latest → search(tag) → downloads → back to search → download visible post
search(tag) → popular → browser back → detail → download
downloads → detail from thumbnail → back → open the same history row again
```

### 6. Performance comes from bounded work and dormant work

Scrolling is controlled by paginated queries, manual history pagination, and
native `content-visibility` for offscreen cards. The playground is lazy-loaded,
throttled, paused when hidden or offscreen, capped for pixel ratio, and has a
non-WebGL fallback.

Rule: every visual experiment needs a lifecycle: mount, visible, hidden,
unmount, and reduced-motion behavior. A beautiful canvas that keeps a render
loop alive after leaving the tab is a resource leak.

### 7. A global tool policy is not automatically a project contract

The local machine had a global uv freshness policy:

```toml
exclude-newer = "7 days"
```

That policy entered `uv.lock` during lock generation, but CI had no equivalent
global configuration. `uv sync --locked` then failed before any project code
ran. The durable fix is to declare the policy in `pyproject.toml` and set
`UV_EXCLUDE_NEWER` in CI, making the clean runner agree with development
without pinning uv itself.

Clean-environment check:

```bash
UV_CONFIG_FILE=/dev/null UV_EXCLUDE_NEWER='7 days' uv sync --locked
UV_CONFIG_FILE=/dev/null UV_EXCLUDE_NEWER='7 days' make check
```

Rule: lock generation must be reproducible without a developer's home
configuration. CI should make security and freshness policy explicit.

### 8. A release candidate needs evidence, not only screenshots

The useful acceptance evidence came from a mix of runtime tests, strict local
gates, browser snapshots, and native build jobs. Headless browser checks are
valuable for DOM, route, accessibility, reduced-motion, and WebGL fallback
behavior; they do not replace native WKWebView/WebView2 interaction evidence.

The agent-browser SOP also matters: use an isolated profile, keep the browser
process scoped to the check, close it, and verify no child process remains.
Resource hygiene is part of correctness for a desktop app.

## Wrong approaches to avoid

| Tried | Why it failed | Better approach |
| --- | --- | --- |
| Trust the visible React card as native download input | route cache and native cache have different lifetimes | rehydrate authoritative post metadata on IPC cache miss |
| Prefer detail cache over the download library | stale/corrupt presentation cache masked a valid local file | library first, cache second, one explicit refresh retry |
| Delete detail cache immediately after promotion | an open inspector may still be decoding that path | keep the cache usable; cleanup owns later removal |
| Count every `Completed`/`ExistingTarget` row as a file | history contains repeated attempts | count unique target paths and explain attempts |
| Let global uv settings silently define the lock | CI has a different home configuration | declare the policy in the project and CI |
| Treat a passing browser capture as native acceptance | WebView behavior and resource lifecycle differ | pair browser evidence with native build and platform checks |

## Working checklist for the next slice

Before changing a cross-layer feature:

1. Name the owner and lifetime of each piece of state.
2. Write the smallest route or lifecycle regression shape.
3. Test the cache-miss and restart path, not only the happy path.
4. Run the clean-toolchain install before trusting local checks:
   `uv sync --locked` and `bun install --frozen-lockfile`.
5. Run `make check`, then `make validate` before requesting review.
6. Update the nearest state contract, verification receipt, and this
   retrospective when a new durable lesson appears.

The 0.1.2 goal was not to finish every experiment. It was to learn where a
playable, attractive desktop toy becomes reliable: at ownership boundaries,
under navigation, during recovery, and on a clean machine.
