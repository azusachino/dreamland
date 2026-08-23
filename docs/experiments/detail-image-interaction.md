# Detail image interaction

## Hypothesis

An artwork should feel inspectable and tangible without turning the detail
view into a game or hiding the image behind a decorative effect.

## Intended feeling

A quiet threshold: pointer movement gives the image a little presence, while
zoom remains deliberate and understandable.

## Implementation

- Pointer position applies a small X/Y tilt directly to the image frame.
- The frame is keyboard-focusable; `+`/`=` and `ArrowUp` zoom in, `-`/`_` and
  `ArrowDown` zoom out, and `0` resets to 100%.
- Explicit zoom controls expose the same state to pointer and keyboard users.
- Zoom is bounded from 100% to 250% and resets when the selected post changes.
- Reduced motion disables tilt but keeps zoom, focus, controls, and the image
  fallback available.
- The image remains an ordinary decoded `<img>` with the existing cached-path
  and decode-error behavior.

## Performance budget

- no React state updates during pointer movement;
- CSS transforms only for tilt/zoom;
- no extra image fetches or textures;
- no feed, query, or download state changes from the interaction.

## What worked

The interaction is additive: users can inspect the image statically, use the
controls, or ignore the effect. The same image source and fallback remain the
source of truth.

## What failed

Runtime input-latency and cross-WebView visual checks are still pending. The
current environment can prove type/build/native gates but cannot provide a
desktop display receipt.

## Decision

**Keep as the CSS-first playable interaction for 0.1.2.** Do not add more
detail effects until desktop checks show that tilt and zoom remain calm rather
than distracting.
