#!/usr/bin/env python3
"""Run the platform-aware Dreamland quality checks."""

from __future__ import annotations

import platform
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def run(label: str, *command: str) -> None:
    print(f"==> {label}")
    subprocess.run(command, cwd=ROOT, check=True)


def main() -> int:
    run("uv lock", "uv", "lock", "--check")
    run("tooling syntax", sys.executable, "-m", "compileall", "-q", "scripts")
    run("frontend build", "bun", "run", "build")
    run(
        "Rust formatting",
        "cargo",
        "fmt",
        "--manifest-path",
        "src-tauri/Cargo.toml",
        "--",
        "--check",
    )

    if platform.system() == "Linux":
        print("==> native Rust tests (skipped: Linux is unsupported)")
    else:
        run("native Rust tests", sys.executable, "scripts/test.py")

    if platform.system() == "Darwin":
        run("Nix flake", "nix", "flake", "check", "--all-systems")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
