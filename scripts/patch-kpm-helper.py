#!/usr/bin/env python3
"""Patch kpm-helper.py for Python 3.9 compatibility."""

from __future__ import annotations

import sys
from pathlib import Path


def patch(path: Path) -> None:
    text = path.read_text(encoding="utf-8")
    old = 'print(f"[ERR] Manifest versions must match, got package manifest v{manifest["manifest_version"]}, got repo manifest v{repositoryManifest[\'manifest_version\']}")'
    new = 'print(f"[ERR] Manifest versions must match, got package manifest v{manifest[\'manifest_version\']}, got repo manifest v{repositoryManifest[\'manifest_version\']}")'
    if old not in text:
        if new in text:
            pass
        else:
            raise SystemExit(f"expected kpm-helper pattern not found in {path}")
    else:
        text = text.replace(old, new)

    deps_old = '"dependencies": manifest["dependencies"]'
    deps_new = '"dependencies": manifest.get("dependencies", [])'
    if deps_old in text:
        text = text.replace(deps_old, deps_new)

    path.write_text(text, encoding="utf-8")


if __name__ == "__main__":
    patch(Path(sys.argv[1]))
