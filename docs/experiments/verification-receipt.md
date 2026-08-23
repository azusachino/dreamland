# 0.1.2 verification receipt

Date: 2026-08-23

| Area | Evidence | Result |
| --- | --- | --- |
| TypeScript/frontend | `make check` | pass |
| Rust formatting/lint/tests | `make check` | pass |
| macOS native packaging | `make build` | pass; arm64 app and DMG produced |
| macOS packaged launch | `open target/release/bundle/macos/Dreamland.app` | pass; native process started; screen capture and assistive-access queries are unavailable in this environment |
| Headless WebGL route | `agent-browser` open/snapshot plus Chrome capture of `#/playground?demo=1` | pass; populated canvas and accessible node controls rendered |
| Headless non-WebGL route | `agent-browser --args '--disable-gpu,--disable-software-rasterizer'` | pass; fallback rendered and remained actionable |
| reduced motion | `agent-browser set media dark reduced-motion` and reload | pass at browser behavior level; native visual check pending |
| Downloads media collection | `agent-browser` open `#/downloads?demo=1`, full screenshot and snapshot | pass; five post records plus archive fallback, progress, summary metrics, existing-target state, and failed state rendered |
| Downloads list/card switch | `agent-browser` click `cards`, inspect `.download-list-cards` and computed progress track | pass; all three download groups switch to cards and the prominent track renders at 0.48rem |
| Download progress semantics | demo queue with one known and one unknown total | pass; overall progress reports known bytes honestly and calls out the unknown item instead of inventing a total |
| Streamed download progress | runtime progress callback test with a four-byte HTTP fixture | pass; the queue receives `0/4` metadata and `4/4` completion updates during the stream rather than only at finish |
| Existing-target history | demo `ExistingTarget` record and runtime enqueue path | pass; “on disk” is retained as attempt history with one concise no-overwrite explanation and active duplicate requests return the original queue record |
| Same-post download affordance | feed/detail card bound to active or completed download records | pass; active posts show queued/downloading state, completed posts show “downloaded”, and existing targets show “on disk” instead of another blind download action |
| Download history preview | completed history row with a local target and delayed post metadata | pass; the local file is preferred for the thumbnail, remote preview is a fallback, and selecting the thumbnail reuses the detail inspector after metadata hydration |
| Notification center | multiple completion/status events in one session | pass; the top-right center keeps up to four stacked notices with independent dismissal and expiry |
| Explore media fallback | `agent-browser` open `#/latest?demo=1` | pass; four deterministic preview cards rendered with disabled demo actions |
| Inspect stage sizing | `agent-browser` click demo post and inspect `.MuiDialog-container` bounds | pass; desktop renders a media stage beside a detail rail instead of a shallow stacked sheet |
| Inspect compact layout | `agent-browser` set viewport 390×844 and capture the demo detail | pass; stage, actions, tags, and metadata stack in the intended order; close control remains reachable |
| Inspect keyboard zoom | focus `.detail-image-frame`, press `+`, inspect zoom status | pass; zoom changed to 110% without losing focus |
| Inspect pointer zoom | click `button[aria-label="zoom in"]`, inspect zoom status and computed transform | pass; navigation no longer intercepts the zoom control; zoom changed to 110% and the image transform changed |
| Inspect wheel/double-click zoom | `agent-browser` wheel input and double-click on `.detail-image-frame` | pass; wheel changes zoom, double-click toggles 150%, and reset returns to 100% |
| Inspect image pan | `agent-browser mouse down/move/up` on a zoomed `.detail-image-frame` | pass; image movement is reflected in bounded `--detail-pan-x/y` values and reset clears both axes |
| Inspect transient detail chrome | open detail and wait for the navigation count timeout | pass; the `1 of N` counter fades after 2.4 seconds while zoom controls remain discoverable |
| Inspect direct route | open `#/posts/yandere/8421?demo=1` without history state | pass; route reconstructs a post reference and opens the detail surface |
| light/reduced-motion quality | `agent-browser set media light reduced-motion` on Downloads demo | pass; hierarchy and progress remain readable without animation |
| opaque surfaces | `--glass-fallback` and forced non-WebGL capture | pass at browser level |
| WebGL context loss | `agent-browser eval` dispatch of `webglcontextlost`, fallback text, and canvas count | pass in browser; native event injection pending |
| playground keyboard path | `agent-browser focus` + `press Enter` on a node button | pass; `aria-pressed=true` and live caption updated; native window check pending |
| repeated playground lifecycle | `agent-browser` route sequence playground → downloads → playground | pass; canvas remounted on both entries; native leak check pending |
| bundle isolation | production build output | pass; normal entry remains ~83 kB minified while the ~518 kB / ~131 kB gzip playground chunk stays lazy; known warning accepted for the isolated experiment |
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

## Browser interaction receipt

On 2026-08-23, the local Vite surface was exercised with the workstation's
`agent-browser` CLI through `bunx` (no project dependency was added). The
accessibility snapshot exposed the playground controls and four preview nodes.
Clicking the first node changed the live caption to `preview · running ·
example data only.`; focusing the second node and pressing Enter set
`aria-pressed=true` and changed the caption to the queued state.

Dispatching a cancelable `webglcontextlost` event returned the expected
prevented result, rendered the existing WebGL fallback, and reduced the stage
canvas count to zero. A fresh route entry rebuilt one canvas; leaving for
Downloads and returning rebuilt one canvas again. A forced software-disabled
session rendered the same fallback, and reduced-motion emulation preserved
the preview orbit summary. These checks prove browser lifecycle and
interaction behavior; the expanded demo fixtures also make the visual contract
repeatable without a live site or native backend. They do not prove WKWebView
or WebView2 parity.
