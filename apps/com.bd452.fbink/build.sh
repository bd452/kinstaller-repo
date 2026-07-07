#!/usr/bin/env bash
set -euo pipefail

APP_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$APP_ROOT/../.." && pwd)"
FBINK_SRC="$APP_ROOT/vendor/FBInk"
FBINK_VERSION="1.25.0"

# shellcheck source=../../scripts/koxtoolchain.sh
source "$REPO_ROOT/scripts/koxtoolchain.sh"
require_kox

if [[ ! -f "$FBINK_SRC/Makefile" ]]; then
    echo "error: FBInk source missing at $FBINK_SRC" >&2
    echo "Run: git submodule update --init --recursive" >&2
    exit 1
fi

if [[ ! -f "$FBINK_SRC/i2c-tools/Makefile" ]]; then
    echo "error: FBInk vendored submodules missing (i2c-tools)" >&2
    echo "Run: git submodule update --init --recursive" >&2
    exit 1
fi

mkdir -p "$APP_ROOT/package/bin/kindlehf" "$APP_ROOT/package/bin/kindlepw2"

build_platform() {
    local platform=$1
    local cross_tc
    cross_tc="$(kox_prefix "$platform")"
    local tool_bin="$KOX_BASE/x-tools/${cross_tc}/bin"

    echo "==> Building FBInk for $platform"
    make -C "$FBINK_SRC" clean
    PATH="$tool_bin:$PATH" make -C "$FBINK_SRC" kindle strip KINDLE=true CROSS_TC="$cross_tc"
    install -m 755 "$FBINK_SRC/Release/fbink" "$APP_ROOT/package/bin/${platform}/fbink"
}

build_platform kindlehf
build_platform kindlepw2

python3 - "$APP_ROOT/package/manifest.json" "$FBINK_VERSION" <<'PY'
import json
import sys

manifest_path, version_str = sys.argv[1:3]
version = [int(part) for part in version_str.split(".")[:3]]
while len(version) < 3:
    version.append(0)

with open(manifest_path, encoding="utf-8") as file:
    manifest = json.load(file)
manifest["version"] = version
with open(manifest_path, "w", encoding="utf-8") as file:
    json.dump(manifest, file, indent=2)
    file.write("\n")
print(f"Synced manifest version to {'.'.join(str(v) for v in version)}")
PY

"$REPO_ROOT/scripts/pack-app.sh" "$APP_ROOT"
