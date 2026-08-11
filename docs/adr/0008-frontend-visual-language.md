# ADR 0008: Frontend visual language and cross-platform rendering

- Status: Accepted
- Date: 2026-08-11

## Context

Dreamland is a Tauri desktop gallery. Its primary task is visual exploration,
so the existing functional toolbar and card grid are not an acceptable product
surface. The design should learn from current Material 3 and Apple platform
guidance without pretending that a webview can reproduce native platform
controls exactly.

Tauri renders the frontend through WebView2/Chromium on Windows and WKWebView
on macOS. WebKit is tied to the installed macOS version, while WebView2 is an
updatable Chromium-based runtime. CSS blur and transparency therefore cannot
be the only source of hierarchy or contrast.

## Decision

Dreamland uses a layered visual language:

1. **Material 3 Expressive is the content system.** Color roles, typography,
   spacing, shape, states, cards, chips, tabs, progress, and adaptive feed
   layouts are implemented as Dreamland-owned design tokens and components.
2. **Liquid Glass is a restrained shell treatment.** It may be used for the
   top app bar, navigation rail, inspector/drawer, and transient popovers. It
   must not be applied to image cards or the scrolling content layer.
3. **Opaque fallback is mandatory.** Every glass surface has a solid Material
   surface fallback when `backdrop-filter` is unavailable, disabled, or harms
   legibility. Native transparent Tauri windows and macOS private APIs are not
   required.
4. **The layout is adaptive.** Expanded windows use a feed plus supporting
   detail pane; compact windows use a single feed and a modal/sheet detail view.
   Settings is an in-place inspector/drawer rather than a separate settings
   page.
5. **Compatibility is functional-first.** The supported desktop baseline is
   macOS Catalina 10.15+ and Windows 10/11, with visual parity targeted at
   current macOS and Windows 11. The UI does not depend on SF Symbols, a
   particular system font, native Liquid Glass, or platform-specific window
   transparency.
6. **Accessibility and state clarity are non-negotiable.** Reduced motion,
   increased contrast, keyboard navigation, visible focus, and non-color-only
   states are supported. Loading, rate-limit, empty, auth, and download states
   remain readable on opaque and translucent surfaces.

## Component/library policy

- Keep React, TanStack Query, Vite, Bun, and Tailwind CSS.
- Use CSS custom properties for Dreamland's Material-derived design tokens;
  Tailwind is a layout and utility tool, not the visual source of truth.
- Use Radix primitives incrementally for dialogs, drawers, tabs, menus,
  popovers, and tooltips. Their behavior/focus logic is adopted, but their
  unstyled output receives Dreamland's own visual treatment.
- Do not adopt `@material/web`: its current documentation places the project
  in maintenance mode.
- Do not adopt MUI for the primary visual system: its current documentation
  identifies Material Design 2 as its supported design language.
- Do not add a carousel or glass-card library.

## Consequences

- Dreamland gets a distinctive visual identity instead of an unmodified
  Material or Apple clone.
- The visual system has more implementation work than a stock component
  theme: tokens, fallbacks, responsive states, and visual regression checks
  must be maintained.
- macOS and Windows may differ subtly in blur, font rasterization, scrollbar,
  and native window behavior; the opaque fallback keeps the interaction model
  stable.
- Performance work must treat the image grid as the hot path: glass effects
  stay out of repeated cards, and large result sets will eventually need
  virtualization.

## Verification gate

Before calling the frontend visual slice complete, verify:

- macOS WKWebView and Windows WebView2 screenshots at compact and expanded
  window sizes;
- blur-enabled and forced-opaque rendering;
- light/dark, reduced-motion, keyboard/focus, and high-contrast states;
- settings drawer, search suggestions, detail pane, and download feedback;
- scrolling performance with a large image result set.

## References

- [Material 3](https://m3.material.io/)
- [Material canonical layouts](https://m3.material.io/foundations/layout/canonical-examples/overview)
- [Apple Liquid Glass](https://developer.apple.com/documentation/TechnologyOverviews/liquid-glass)
- [Apple materials](https://developer.apple.com/design/human-interface-guidelines/materials)
- [Tauri webview versions](https://v2.tauri.app/reference/webview-versions/)
- [Radix Primitives](https://www.radix-ui.com/primitives/docs/overview/introduction)
