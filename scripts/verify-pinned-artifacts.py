#!/usr/bin/env python3
"""Verify descriptor hashes for artifacts hosted from this repository."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from urllib.parse import urlparse


REPO_ROOT = Path(__file__).resolve().parent.parent
PAGES_HOST = "bd452.github.io"
PAGES_PREFIX = "/kinstaller-repo/"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as file:
        for chunk in iter(lambda: file.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> None:
    checked = 0
    for descriptor_path in sorted((REPO_ROOT / "registry" / "sources").glob("*.json")):
        descriptor = json.loads(descriptor_path.read_text(encoding="utf-8"))
        for package in descriptor.get("packages", []):
            artifact = package["artifact"]
            parsed = urlparse(artifact["url"])
            if parsed.netloc != PAGES_HOST or not parsed.path.startswith(PAGES_PREFIX):
                continue
            relative = parsed.path.removeprefix(PAGES_PREFIX)
            local_path = REPO_ROOT / relative
            if not local_path.is_file():
                raise SystemExit(f"missing pinned artifact: {local_path}")
            if local_path.stat().st_size != artifact["size"]:
                raise SystemExit(f"size mismatch for {local_path}")
            if sha256(local_path) != artifact["sha256"]:
                raise SystemExit(f"SHA-256 mismatch for {local_path}")
            checked += 1
    print(f"Verified {checked} repository-hosted artifact(s).")


if __name__ == "__main__":
    main()
