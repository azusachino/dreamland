# Dreamland 0.1.2 plan

Status: active. The first transition, download-state, and isolated-playground
experiments are implemented; 0.1.2 is now expanding into a full design-language
pass across Explore, Inspect, Keep, and the playground shell. Desktop/WebView
acceptance and final cross-platform performance evidence remain open. This is
a quality and learning milestone, not a promise to add every experiment
described here.

## Intention

Make the state of the dream visible.

Dreamland should remain a fast, dependable image gallery while becoming a
playable personal toy: browsing, inspecting, and keeping images should each
have a distinct visual response. The experimental parts must be isolated so a
failed rendering experiment does not damage the normal gallery workflow.

The 0.1.2 success bar is therefore two-dimensional:

- the core gallery is clearer, calmer, more accessible, and no slower;
- at least one small playable experiment teaches us something concrete about
  interaction, rendering, or performance.

## Principles

- Make state visible before making it decorative.
- Use motion to explain a change, never merely to fill space.
- Prefer CSS and existing components before adding a rendering dependency.
- Keep the image grid on the cheap path; experiments belong in isolated
  surfaces or routes.
- Treat reduced motion, keyboard input, focus, opaque fallbacks, and WebView
  differences as part of the design rather than later polish.
- Record failures and removals as useful project knowledge.

## Design-language contract for the quality pass

The user should be able to read Dreamland's state at a glance:

```text
discover  →  inspect  →  keep
ambient      focused     tangible
```

The pass will tune existing components around four shared rules:

- **Media leads.** A post thumbnail or a deliberate fallback carries the
  identity of a download; full/detail media stays on the cache-owned path.
- **State has a voice.** Queued, running, kept, failed, unavailable, and
  loading states use shared names, contrast, icon/text support, and the same
  motion grammar.
- **Surfaces have hierarchy.** Glass is reserved for shell/panel surfaces;
  repeated cards and download rows use opaque, cheap containers.
- **Experiments have a room.** The playground can be expressive and WebGL
  backed, but Explore, Inspect, and Keep remain usable if WebGL is absent.

### Assumptions made for this pass

1. `DownloadRecord.metadata` is the correct presentation contract for a
   lightweight preview; no new Rust command or filesystem access is needed.
2. A deterministic `?demo=1` fixture is useful for visual QA and learning, but
   demo actions must not imply that a real download was changed.
3. The existing MUI boundary remains; the quality pass tunes the theme and
   local primitives before considering a component-library migration.
4. Three.js remains lazy and isolated to the playground; adding 3D effects to
   the feed or download list would spend performance budget without proving
   user value.

### Acceptance bar for the expanded pass

- Downloads show a lightweight preview, metadata hierarchy, progress, and
  honest fallback for missing/failed media.
- Explore cards, navigation, panels, and feedback states share the same
  spacing, border, focus, status, and motion vocabulary.
- Inspect keeps its cache-owned full-image path, readable loading/failure
  states, keyboard controls, and reduced-motion behavior.
- `#/downloads?demo=1` and `#/playground?demo=1` are deterministic visual
  fixtures that can be inspected without a live site or native backend.
- The normal feed bundle does not import Three.js; the playground remains
  lazy-loaded and resource-clean.
- `make check` passes, and agent-browser receipts cover the new Downloads
  surface in light/dark and reduced-motion modes where the native desktop
  display is unavailable.

## Big steps

### 1. Establish the Dreamland transition system

This is the first implementation step.

The current UI already has useful motion, but durations and behaviors are
scattered through CSS. We will turn them into a small, named motion language
before adding new effects.

#### Scope

- audit existing transitions and keyframes in `src/styles.css` and the MUI
  overrides;
- define motion tokens for fast feedback, standard movement, emphasis, and
  reduced-motion behavior;
- define a small set of reusable patterns:
  - skeleton to content reveal;
  - page/feed replacement;
  - detail open and close;
  - text-state swap;
  - number change/pop-in;
  - spinner to completion check;
  - restrained toast entrance;
- apply the first patterns to the highest-value flows:
  - feed loading and refresh;
  - post detail open/close;
  - download queued/running/completed/failed;
  - favorite add/remove;
- keep transitions CSS-first and use the existing `prefers-reduced-motion`
  fallback;
- use the transitions.dev catalog as a reference, without installing an
  additional agent skill or animation framework.

#### Not in scope

- a new animation library;
- animation on every card or every state update;
- Three.js;
- decorative background loops;
- changing the product's layout or component ownership.

#### Verification

- every new motion has a reduced-motion behavior;
- the same state change uses the same motion pattern across the app;
- keyboard and focus behavior remain unchanged;
- the UI remains readable with forced-opaque surfaces;
- screenshots cover feed loading, detail open/close, and download state
  changes in light and dark modes.

### 2. Make downloading visibly understandable

The runtime already stores `bytes_downloaded` and `total_bytes`, and the
frontend polls download state. This step makes that state useful to a person.

#### Scope

- show per-item progress when a total is known;
- show an indeterminate state when a total is unavailable;
- show an overall progress summary for active downloads;
- distinguish queued, running, completed, existing, failed, and cancelled
  states without relying on color alone;
- use the transition system for status changes and completion;
- keep retry, cancel, and open-file actions stable while progress updates.

#### Verification

- progress updates do not reorder or restart rows;
- terminal states remain visible in history;
- unknown totals never display false percentages;
- polling and rendering remain cheap with a large history;
- queue behavior remains runtime-owned and unchanged.

### 3. Improve the three core surfaces

Make the product's main loop feel intentional without introducing new
technology.

#### Explore

- make site, query, popular window, and result state easy to read;
- give loading, empty, error, and unsupported states a consistent voice;
- refine card hover, focus, selection, and favorite feedback;
- preserve the fast scrolling path and keep glass effects out of repeated
  content cards.

#### Inspect

- make entering and leaving the detail stage feel like crossing a threshold;
- improve image loading and fallback communication;
- keep metadata hierarchy calm and scannable;
- retain keyboard navigation and route restoration.

#### Keep

- make the Downloads panel feel like a collection in progress rather than a
  raw job log;
- connect completion feedback to the resulting history item;
- keep notifications concise and non-duplicative.

### 4. Add one CSS-first playable interaction

Before adopting WebGL, prove that the interaction itself is valuable.

The first candidate is an interactive detail image: gentle tilt/parallax,
zoom, and reset behavior, driven by pointer and keyboard input. It should make
the artwork feel tangible without making inspection harder.

#### Verification

- pointer, keyboard, and static fallback all expose the same image;
- reduced motion removes tilt and preserves zoom/accessibility;
- no layout shift or input lag appears during interaction;
- the interaction does not affect the feed or download queue.

### 5. Create an isolated Three.js playground

Only after the transition and CSS-first slices are understood should Three.js
enter the application.

Three.js experiments live behind a lazy-loaded playground surface. They do not
become a dependency of the normal feed path or the core gallery layout.

#### First experiment: download constellation

- active downloads become a small number of image tiles or objects;
- progress affects a visible property such as fill, glow, or distance;
- completed downloads settle into a kept cluster;
- selecting an object returns to the normal post/download workflow.

#### Performance rules

- one canvas maximum per visible route;
- use preview/sample assets, never full-size originals;
- cap pixel ratio and animation rate;
- pause when hidden or inactive;
- render on demand when continuous animation is not needed;
- dispose textures, materials, geometries, and listeners on unmount;
- provide a non-WebGL and reduced-motion fallback.

#### Exit criteria

Keep the experiment only if it improves the feeling of collecting or teaches
something reusable. Otherwise, record the failure and remove it without
affecting the gallery.

### 6. Verify quality across the real desktop targets

The visual language is not complete when it looks good in one development
window.

#### Verification matrix

- macOS WKWebView and Windows WebView2;
- compact and expanded layouts;
- light and dark themes;
- reduced motion and high contrast;
- keyboard-only navigation and visible focus;
- forced-opaque rendering;
- large image result sets and long download histories;
- entering and leaving the playground repeatedly to detect resource leaks.

Run the normal project gate with `make check` after each completed slice that
changes code.

### 7. Keep a learning record

Each experiment gets a short note containing:

```text
hypothesis
intended feeling
implementation
performance budget
what worked
what failed
decision: keep / revise / remove
```

The learning record is part of the deliverable. A removed experiment is a
successful result if it clarified the product or the technical boundary.

## 0.1.2 not doing

- no new site adapter;
- no broad component-library migration;
- no permanent animated particle background;
- no 3D effects on every gallery card;
- no local bookmark or creator-profile expansion;
- no feature that weakens the existing runtime/frontend boundary;
- no visual polish that hides incomplete release-gate behavior.

## First slice definition of done

Step 1 is complete when the app has named motion tokens, the first shared
transition patterns are applied to loading/detail/download states, reduced
motion and opaque fallback are verified, and the result is documented with
before/after screenshots or a short manual receipt.
