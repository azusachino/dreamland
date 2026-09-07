# Android asset-loading and detail-view audit

Status: audit, 2026-09-07. Written after Android on-device testing surfaced
"full image could not be decoded" in the detail view and a general "slow
network" feeling, and a request to compare the detail mechanism against the
vendored `vendor/moebooru` reference (`com.github.yueeng.moebooru`, also
installed on-device as the yande.re flavor
`com.github.yueeng.moebooru.yande`).

**Finding 1 is fixed** (the `$APPCACHE` scope entry proposed below was
applied and verified on-device: detail images load without the
refresh-then-fail loop). Finding 2 (the tag/metadata panel restructure) is
still open -- not implemented.

## Finding 1 (confirmed, high confidence): asset-protocol scope mismatch breaks every Android detail-image load

### The bug

`src-tauri/tauri.conf.json`'s `assetProtocol.scope` is:

```json
["$CACHE/dreamland/detail/**/*", "$HOME/.cache/dreamland/detail/**/*"]
```

`$CACHE` and `$HOME` are Tauri path variables resolved by
`tauri::path::PathResolver`. On Android, per the actual installed crate
source (`~/.cargo/registry/.../tauri-2.11.5/src/path/android.rs`):

```rust
pub fn cache_dir(&self) -> Result<PathBuf> {
    self.call_resolve("getExternalCacheDir")   // $CACHE -> external storage
}
pub fn app_cache_dir(&self) -> Result<PathBuf> {
    self.call_resolve("getCacheDir")           // $APPCACHE -> internal storage
}
```

`$CACHE` (`getExternalCacheDir`) and `$APPCACHE` (`getCacheDir`) are **two
different directories** on Android — external vs. internal app storage.
ADR 0011's mobile path-resolution fix
(`set_mobile_dirs` in `src-tauri/src/lib.rs`) sets `XDG_CACHE_HOME` to
`app.path().app_cache_dir()` — i.e. `getCacheDir()`, internal storage — which
`dreamland-runtime::default_cache_root()`/`default_detail_cache_path()`
consult first. So on Android, the detail-image cache is written under
`getCacheDir()/dreamland/detail/...`, but the asset-protocol scope only
allow-lists `getExternalCacheDir()/dreamland/detail/**/*`. The two never
match. `$HOME` is a desktop/Linux-XDG-only fallback and resolves to nothing
meaningful on Android either.

### Why `getCacheDir()` (internal) is still the right choice, not the bug to reverse

`getExternalCacheDir()` requires external storage to be mounted and can
return null (no SD card / unavailable emulated storage); `getCacheDir()` is
always available and is Android's documented recommendation for app-private
caches that don't need user visibility. The fix belongs in the scope
declaration (add the `$APPCACHE` variant), not in reverting the path source.

### The failure sequence this produces (`src/App.tsx:2112-2162`)

1. `loadDetailImage` resolves/creates a real, valid cached file under
   `getCacheDir()/dreamland/detail/...` and returns its path.
2. The frontend calls `convertFileSrc(path)` and sets it as `<img src>`.
3. The WebView's `asset://` handler checks the path against
   `assetProtocol.scope`, finds no match, and refuses to serve it — `onerror`
   fires immediately.
4. The `onerror` handler (first failure only) calls `loadDetailImage(...,
   refresh=true)` — a **full network re-download** of the original image,
   re-cached to the same (still out-of-scope) directory.
5. The new path hits the identical scope mismatch — `onerror` fires again.
6. `refreshed` is now `true`, so the handler gives up and renders "full image
   unavailable / the full image could not be decoded".

Net effect, on **every single detail view opened on Android**: one instant
local-read failure, one full wasted network image download that still fails
to display, and a permanent error state. This is a concrete, mechanical
explanation for both symptoms reported — the visible detail-view breakage,
and "slow network" (every detail open silently re-downloads the full-size
image over the network and then throws the result away).

### A second, lower-priority instance of the same class of gap (not Android-specific, not a regression)

`DownloadThumbnail` (`src/App.tsx:2369-2374`) also calls
`convertFileSrc(record.target_path)` for a completed download's file.
`target_path` lives under the user-configured `download_path`, which is
**not** in `assetProtocol.scope` on any platform (it can't be, statically —
the user can point it at any folder they want on desktop). Unlike the detail
cache, this component already has a designed graceful fallback —
`sources = [localSource, preview_url, sample_url, full_url]` with
index-advance on error — so it degrades to a network image instead of
breaking outright. This has presumably always cost a network fetch for
download-history thumbnails on every platform; worth knowing about, not
urgent, and not something this port introduced.

### Proposed fix (not yet applied)

Add `"$APPCACHE/dreamland/detail/**/*"` as a third entry in
`assetProtocol.scope`. Additive, platform-safe: desktop never sets
`XDG_CACHE_HOME`, so `dreamland-runtime` never resolves paths under
`app_cache_dir()` there — the new entry is inert on desktop and only takes
effect where the mobile path redirection actually applies.

## Finding 2: detail-view mechanism, structural comparison with `vendor/moebooru`

Read from `vendor/moebooru/app/src/main/java/com/github/yueeng/moebooru/PreviewActivity.kt`
and live on-device testing of the installed `yande` app (same codebase, yande.re flavor).

| | `vendor/moebooru` (`PreviewActivity.kt`) | Dreamland (`PostInspector`, `App.tsx`) |
| --- | --- | --- |
| Image loading | Native `Glide`, its own memory+disk cache; no custom cache path, no protocol/scope layer between the image and the `ImageView` | Rust downloads/caches to disk, frontend loads it through Tauri's `asset://` protocol + a static allow-list scope — an extra layer with its own failure mode (Finding 1) |
| Progressive loading | `thumbnail(Glide...load(preview_url))` cross-fades into the full `sample_url` load — the low-res preview is visible immediately while the higher-res version streams in | Skeleton placeholder, then the full image appears once fully loaded; no progressive/blur-up step from the already-available `preview_url`/`sample_url` |
| Paging between posts | `ViewPager2` — native horizontal swipe between posts in the pool/feed, `onPageSelected` drives which post's data loads | `canGoPrevious`/`canGoNext` with explicit prev/next controls (arrow buttons); no swipe-to-advance gesture identified in the current implementation |
| Tag/metadata disclosure | `BottomSheetBehavior` sliding panel, toggled by a single tap on the image; tags render in a `RecyclerView` grouped and colored by type (character/copyright/artist/general) with a small type-label over a larger tag-name, per Finding-adjacent screenshot evidence | Tags render inline below the image in an always-expandable `.tag-list` of same-style chips (color-coded by tone already, per `tagTone()`), plus separate collapsible "related tags" and "more data" sections — three different disclosure affordances rather than one unified panel |
| Zoom/pan | Not inspected in this pass | Custom pointer-drag + wheel zoom/pan implemented in `PostInspector` (`handleWheel`, `handlePointerDown`, tilt effect) |

This is a real architectural difference, not just styling: moebooru's tag
panel is one coherent, typed, groupable surface; Dreamland's is three
separate flat-chip sections. Whether to converge on a single grouped-tag
panel (matching moebooru) is a design decision with real scope (touches the
tag-type data already present via `tagTone()`, the related-tags query, and
the "more data" section's contents) — flagged here for review, not
implemented.

## Open decision for the user

1. Apply the one-line `$APPCACHE` scope fix (Finding 1) — mechanical,
   high-confidence, addresses both reported symptoms directly.
2. Whether/how far to restructure the detail view's tag/metadata disclosure
   toward moebooru's single grouped-panel model (Finding 2) — a real design
   decision, not a bug fix, scoped separately from item 1.
