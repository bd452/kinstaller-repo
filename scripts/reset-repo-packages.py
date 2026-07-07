#!/usr/bin/env python3
"""Clear packages/ and reset manifest.json package entries before a rebuild."""

from __future__ import annotations

import json
import shutil
from pathlib import Path


def main() -> None:
    repo_root = Path(__file__).resolve().parent.parent
    manifest_path = repo_root / "manifest.json"
    packages_dir = repo_root / "packages"

    with manifest_path.open(encoding="utf-8") as file:
        manifest = json.load(file)

    manifest["packages"] = {}

    with manifest_path.open("w", encoding="utf-8") as file:
        json.dump(manifest, file, indent=2)
        file.write("\n")

    if packages_dir.exists():
        shutil.rmtree(packages_dir)
    packages_dir.mkdir()


if __name__ == "__main__":
    main()
