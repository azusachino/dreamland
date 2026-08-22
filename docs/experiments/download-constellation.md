# Download constellation

## Hypothesis

Representing downloads as a small spatial cluster can make the act of keeping
art feel more tangible than a log of rows, while leaving the normal download
workflow as the source of truth.

## Intended feeling

Quietly magical and inspectable: work grows while it is active, then settles
into a kept cluster when complete.

## Implementation

- A lazy-loaded `DownloadPlayground` owns one Three.js canvas.
- It uses geometry only; full-size originals and remote image textures never
  enter the experiment.
- Progress changes object scale, active objects drift gently, and completed
  objects settle into a cluster.
- Visibility pauses the animation loop; reduced motion renders a static frame.
- WebGL setup failures expose a normal download-panel fallback.
- `#/playground?demo=1` exposes a clearly labeled local preview fixture for
  visual study; it never enters IPC, queue, or history.
- The canvas is paired with native focusable node controls that expose each
  record's identity, status, and progress without requiring pointer input.
- Selecting a live node exposes a direct handoff back to Downloads; preview
  nodes remain explicitly non-persistent.
- Every geometry, material, renderer, observer, and listener is disposed when
  the surface unmounts.

The renderer follows the Three.js guidance to cap pixel ratio, resize from the
CSS display size, use `setAnimationLoop`, and dispose GPU resources when the
surface is no longer used:

- <https://threejs.org/manual/en/responsive.html>
- <https://threejs.org/docs/pages/WebGLRenderer.html>

## Performance budget

- one canvas maximum;
- at most 24 recent queued, running, or completed records;
- pixel ratio capped at 1.5;
- active animation capped at 30 frames per second;
- low-power WebGL preference;
- completed-only scenes render once; no continuous loop while hidden or in
  reduced-motion mode.

## What worked

The experiment has a narrow ownership boundary: it reads download records and
returns to the existing Downloads surface. It does not alter queue semantics,
IPC, cache paths, or post detail behavior.

The `/playground` route was captured at 1440×1000 in headless Chrome in two
modes. With normal headless rendering, the WebGL canvas initialized and the
empty state rendered. With WebGL explicitly disabled, the shell, selected
navigation state, and non-WebGL recovery action rendered coherently. This
verifies both browser-level branches, not native WebView2/WKWebView parity or
populated live-node interaction; browser IPC has no download fixtures here.

The local preview fixture was also captured with four nodes. Halos, state
legend, relative node size, and the kept-cluster arrangement read clearly at
desktop size; the native node controls provide a keyboard-verifiable path for
the same selection state, while the screenshot-only probe did not exercise
pointer selection.

## What failed

Runtime visual evidence is still pending on macOS and Windows. The current
headless environment can verify compilation and cleanup paths, but cannot prove
that the constellation feels useful at a real desktop window size.

## Decision

**Keep as an experiment for 0.1.2; do not promote it into the gallery yet.**
The interaction is useful enough to retain behind the playground boundary;
revisit promotion only after manual checks of motion quality, WebGL fallback,
repeated route entry/exit, and native WebView behavior.
