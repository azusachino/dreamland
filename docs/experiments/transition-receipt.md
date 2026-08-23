# Transition receipt

Date: 2026-08-23

## Implemented surface

| State change | Visible response | Reduced-motion behavior |
| --- | --- | --- |
| feed/view replacement | content lift/fade and first-card stagger | immediate content, no delay |
| card inspection | hover lift, image scale, press response | transitions collapse to immediate |
| detail open | backdrop fade plus panel threshold motion | immediate opaque panel |
| detail image load | skeleton shimmer to decoded-image reveal | static skeleton/image swap |
| download thumbnail load | compact skeleton to preview reveal, explicit unavailable fallback | immediate preview/fallback |
| download state | status pop, progress width, indeterminate movement | immediate status/width |
| favorite mutation | color transition and favorite pop | immediate color/icon state |
| toast arrival | short top-edge entrance | immediate toast |

## Evidence

- Motion is named in `src/styles.css` with fast, standard, emphasis, loading,
  shimmer, easing, and semantic state tokens.
- Gallery cards and download rows use the same skeleton-to-content reveal and
  explicit media fallback language; Downloads now keeps the preview visible
  alongside progress and target metadata.
- The MUI full-screen detail container is explicitly width-bound so the
  Inspect stage stays a desktop surface instead of collapsing to content width.
- `prefers-reduced-motion: reduce` collapses animation duration, delay, and
  transition duration.
- Shell surfaces retain solid `--glass-fallback` backgrounds when blur is not
  available.
- `make check` passed after the final transition changes on 2026-08-23.
- The normal feed path uses no Three.js import; the experiment is a lazy route.
- The playground caps renderer pixel ratio, pauses for hidden tabs, reduced
  motion, and offscreen stages, and disposes its renderer-owned resources on
  unmount or context loss.
- Headless Chrome captured the `/playground` route at 1440×1000 with the
  forced non-WebGL fallback; shell hierarchy and recovery action were legible.
- A second headless capture without GPU disabling initialized the WebGL canvas
  and rendered the empty-state overlay at the same size.
- `agent-browser` captured deterministic `?demo=1` fixtures for Explore,
  Inspect, Downloads, and the playground. The Downloads fixture showed four
  previews across running, queued, completed, and failed states plus a pool
  archive; the reduced-motion light capture showed the same hierarchy without
  animated dependence.

## Open runtime receipt

Desktop screenshots and interaction checks on macOS WKWebView and Windows
WebView2 remain pending. The current execution environment has no capturable
display, so this document does not claim visual parity from build output alone.
