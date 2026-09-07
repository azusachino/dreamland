#!/usr/bin/env python3
"""Patch a known upstream wry bug in the generated Android project.

wry 0.55.1's RustWebView.kt declares getCookies(url): String (non-null) but
backs it with CookieManager.getCookie(url), which returns null whenever no
cookie exists yet for a URL -- true on first launch, before any site has set
one. That null trips Kotlin's runtime null-check inside a JNI call from
Chromium's native WebView code, aborting the app (debug builds) or getting
stripped as unreachable by R8 (release builds, NoSuchMethodError). See
docs/adr/0011-android-target.md. src-tauri/gen/ is gitignored and regenerated
by `tauri android init`, so this patch must be re-applied after every fresh
init; this script is idempotent and safe to run unconditionally.
"""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TARGET = (
    ROOT
    / "src-tauri/gen/android/app/src/main/java/icu/azusachino/dreamland/generated/RustWebView.kt"
)
BROKEN = "return cookieManager.getCookie(url)\n"
FIXED = 'return cookieManager.getCookie(url) ?: ""\n'


def main() -> int:
    if not TARGET.exists():
        print(f"skip    {TARGET} not found (run `tauri android init` first)")
        return 0

    text = TARGET.read_text()
    if FIXED in text:
        print(f"ok      {TARGET.relative_to(ROOT)} already patched")
        return 0
    if BROKEN not in text:
        print(
            f"warn    {TARGET.relative_to(ROOT)} does not match the known-bad "
            "getCookies() body; wry's template may have changed upstream -- "
            "check docs/adr/0011-android-target.md",
            file=sys.stderr,
        )
        return 1

    TARGET.write_text(text.replace(BROKEN, FIXED, 1))
    print(f"fixed   {TARGET.relative_to(ROOT)}: getCookies() null-safety")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
