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
- low-power WebGL preference;
- no texture loading or continuous loop while hidden/reduced-motion.

## What worked

The experiment has a narrow ownership boundary: it reads download records and
returns to the existing Downloads surface. It does not alter queue semantics,
IPC, cache paths, or post detail behavior.

## What failed

Runtime visual evidence is still pending on macOS and Windows. The current
headless environment can verify compilation and cleanup paths, but cannot prove
that the constellation feels useful at a real desktop window size.

## Decision

**Keep as an experiment for 0.1.2; do not promote it into the gallery yet.**
Revisit after manual checks of motion quality, WebGL fallback, repeated route
entry/exit, and whether selecting a node helps users understand the download
state.
