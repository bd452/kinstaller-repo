#!/usr/bin/env bash
set -euo pipefail

APP_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$APP_ROOT/../.." && pwd)"
FBINK_MANIFEST="$REPO_ROOT/apps/com.bd452.fbink/package/manifest.json"

python3 - "$APP_ROOT/package/manifest.json" "$FBINK_MANIFEST" <<'PY'
import json
import sys

demo_manifest_path, fbink_manifest_path = sys.argv[1:3]

with open(fbink_manifest_path, encoding="utf-8") as file:
    fbink_version = json.load(file)["version"]

with open(demo_manifest_path, encoding="utf-8") as file:
    demo_manifest = json.load(file)

max_version = [fbink_version[0] + 1, 0, 0]

for dependency in demo_manifest.get("dependencies", []):
    if dependency.get("id") == "com.bd452.fbink":
        dependency["min"] = fbink_version
        dependency["max"] = max_version
        break
else:
    raise SystemExit("com.bd452.fbink dependency not found in demo manifest")

with open(demo_manifest_path, "w", encoding="utf-8") as file:
    json.dump(demo_manifest, file, indent=2)
    file.write("\n")

version = ".".join(str(part) for part in fbink_version)
print(f"Synced com.bd452.fbink dependency to [{version}, {max_version[0]}.0.0)")
PY

python3 "$APP_ROOT/scripts/make-icon.py"
"$REPO_ROOT/scripts/pack-app.sh" "$APP_ROOT"
