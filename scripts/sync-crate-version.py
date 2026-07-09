#!/usr/bin/env python3
"""Sync a package manifest's version from the Cargo workspace version.

Usage: sync-crate-version.py <manifest.json> <path/to/Cargo.toml>

Reads `version = "X.Y.Z"` from the `[workspace.package]` table of the given
Cargo.toml and writes it into the manifest's `version` field as a 3-int array.
Avoids a tomllib dependency (not available on Python 3.9) by scanning the
relevant table directly.
"""

import json
import re
import sys


def read_workspace_version(cargo_toml_path: str) -> list[int]:
    with open(cargo_toml_path, encoding="utf-8") as file:
        lines = file.readlines()

    in_section = False
    for line in lines:
        stripped = line.strip()
        if stripped.startswith("[") and stripped.endswith("]"):
            in_section = stripped == "[workspace.package]"
            continue
        if in_section:
            match = re.match(r'version\s*=\s*"(\d+)\.(\d+)\.(\d+)"', stripped)
            if match:
                return [int(match.group(i)) for i in (1, 2, 3)]
    raise SystemExit(
        f"could not find [workspace.package] version in {cargo_toml_path}"
    )


def main() -> None:
    manifest_path, cargo_toml_path = sys.argv[1:3]
    version = read_workspace_version(cargo_toml_path)

    with open(manifest_path, encoding="utf-8") as file:
        manifest = json.load(file)
    manifest["version"] = version
    with open(manifest_path, "w", encoding="utf-8") as file:
        json.dump(manifest, file, indent=2)
        file.write("\n")

    print(f"Synced {manifest['id']} version to {'.'.join(map(str, version))}")


if __name__ == "__main__":
    main()
