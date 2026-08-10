#!/usr/bin/env python3
"""Run native Dreamland tests on supported desktop hosts."""

from __future__ import annotations

import platform
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def main() -> int:
    if platform.system() == "Linux":
        print("SKIP: native Tauri tests are not run on unsupported Linux hosts")
        return 0

    return subprocess.run(
        [
            "cargo",
            "test",
            "--manifest-path",
            str(ROOT / "src-tauri/Cargo.toml"),
        ],
        cwd=ROOT,
        check=False,
    ).returncode


if __name__ == "__main__":
    raise SystemExit(main())
