#!/usr/bin/env python3
"""Check the host tools required for Dreamland daily development."""

from __future__ import annotations

import platform
import shutil
import sys


def main() -> int:
    system = platform.system()
    tools = ["uv", "bun", "cargo", "rustfmt"]
    if system == "Darwin":
        tools.append("nix")

    missing = [tool for tool in tools if shutil.which(tool) is None]
    for tool in tools:
        state = "ok" if tool not in missing else "missing"
        print(f"{state:>7}  {tool}")

    if system == "Linux":
        print("skip    Linux is outside Dreamland's supported platform matrix")
    elif system not in {"Darwin", "Windows"}:
        print(f"warn    unvalidated host platform: {system}")

    if missing:
        print(f"missing required tools: {', '.join(missing)}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
