# 0.1.2 verification receipt

Date: 2026-08-22

| Area | Evidence | Result |
| --- | --- | --- |
| TypeScript/frontend | `make check` | pass |
| Rust formatting/lint/tests | `make check` | pass |
| macOS native packaging | `make build` | pass; arm64 app and DMG produced |
| macOS packaged launch | `open target/release/bundle/macos/Dreamland.app` | pass; native process started; screen capture and assistive-access queries are unavailable in this environment |
| Headless WebGL route | Chrome capture of `#/playground?demo=1` | pass; populated canvas rendered |
| Headless non-WebGL route | Chrome capture with GPU disabled | pass; fallback rendered |
| reduced motion | source contract plus CSS gate | implemented; native visual check pending |
| opaque surfaces | `--glass-fallback` and forced non-WebGL capture | pass at browser level |
| WebGL context loss | renderer stops, disposes its scene/listeners, and existing Downloads fallback is shown | teardown path implemented; native event injection pending |
| playground keyboard path | native node controls plus live Downloads handoff | implemented; native window check pending |
| bundle isolation | production build output | pass; normal entry remains ~76 kB minified while the ~518 kB / ~131 kB gzip playground chunk stays lazy; known warning accepted for the isolated experiment |
| macOS WKWebView interaction | native window pointer/keyboard pass | pending; no capturable display in this environment |
| Windows WebView2 interaction | native window pass | pending; no Windows runner available |
| long queue/history behavior | existing runtime tests and bounded playground records | pass at code/test level; populated native run pending |

## Interpretation

The implementation and browser-level branches are verified. The packaged
macOS process starts, but this environment cannot capture or inspect its native
window, so that is not visual acceptance. Windows and native interactive macOS
checks remain honest release evidence gaps rather than being inferred from
Chrome or TypeScript output.

The CI workflow now defines a `macos-latest`/`windows-latest` desktop matrix
that runs the repository checks and native Tauri packaging. It is a future
machine-verification path; this branch has not pushed or observed that remote
run yet.
