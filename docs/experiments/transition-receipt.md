# Transition receipt

Date: 2026-08-22

## Implemented surface

| State change | Visible response | Reduced-motion behavior |
| --- | --- | --- |
| feed/view replacement | content lift/fade and first-card stagger | immediate content, no delay |
| card inspection | hover lift, image scale, press response | transitions collapse to immediate |
| detail open | backdrop fade plus panel threshold motion | immediate opaque panel |
| detail image load | skeleton shimmer to decoded-image reveal | static skeleton/image swap |
| download state | status pop, progress width, indeterminate movement | immediate status/width |
| favorite mutation | color transition and favorite pop | immediate color/icon state |
| toast arrival | short top-edge entrance | immediate toast |

## Evidence

- Motion is named in `src/styles.css` with fast, standard, emphasis, loading,
  and shimmer tokens.
- `prefers-reduced-motion: reduce` collapses animation duration, delay, and
  transition duration.
- Shell surfaces retain solid `--glass-fallback` backgrounds when blur is not
  available.
- `make check` passed after the final transition changes on 2026-08-22.
- The normal feed path uses no Three.js import; the experiment is a lazy route.

## Open runtime receipt

Desktop screenshots and interaction checks on macOS WKWebView and Windows
WebView2 remain pending. The current execution environment has no capturable
display, so this document does not claim visual parity from build output alone.
