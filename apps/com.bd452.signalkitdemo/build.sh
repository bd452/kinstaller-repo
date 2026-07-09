#!/usr/bin/env bash
# Cross-compile the SignalKit demo binary for each Kindle platform and stage it
# into the package. Modeled on ../com.bd452.demo/build.sh. The binary links both
# signalkit and FBInk statically, so the package has no runtime dependencies.
set -euo pipefail

APP_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$APP_ROOT/../.." && pwd)"
RUST_DIR="$REPO_ROOT/rust"
FBINK_SRC="$REPO_ROOT/apps/com.bd452.fbink/vendor/FBInk"

# shellcheck source=../../scripts/koxtoolchain.sh
source "$REPO_ROOT/scripts/koxtoolchain.sh"
require_kox

if [[ ! -f "$FBINK_SRC/Makefile" ]]; then
    echo "error: FBInk source missing at $FBINK_SRC" >&2
    echo "Run: git submodule update --init --recursive" >&2
    exit 1
fi

mkdir -p "$APP_ROOT/package/bin/kindlehf" "$APP_ROOT/package/bin/kindlepw2"

build_platform() {
    local platform=$1
    local cross_tc tool_bin rust_target linker_env
    cross_tc="$(kox_prefix "$platform")"
    tool_bin="$(kox_tool_bin "$platform")"
    rust_target="$(kox_rust_target "$platform")"
    linker_env="$(kox_rust_linker_env "$platform")"

    echo "==> Building signalkit-demo for $platform ($rust_target)"
    make -C "$FBINK_SRC" clean >/dev/null 2>&1 || true

    env CROSS_TC="$cross_tc" PATH="$tool_bin:$PATH" \
        "$linker_env=$tool_bin/${cross_tc}-gcc" \
        cargo build --manifest-path "$RUST_DIR/Cargo.toml" \
        -p signalkit-demo --release --features fbink --target "$rust_target"

    # Honor CARGO_TARGET_DIR (e.g. rust/target-kindle from build-in-container.sh).
    local target_dir="${CARGO_TARGET_DIR:-$RUST_DIR/target}"
    install -m 755 "$target_dir/${rust_target}/release/signalkit-demo" \
        "$APP_ROOT/package/bin/${platform}/signalkit-demo"
}

build_platform kindlehf
build_platform kindlepw2

# Sync the package version from the Cargo workspace version.
python3 "$REPO_ROOT/scripts/sync-crate-version.py" \
    "$APP_ROOT/package/manifest.json" "$RUST_DIR/Cargo.toml"

python3 "$APP_ROOT/scripts/make-icon.py"
"$REPO_ROOT/scripts/pack-app.sh" "$APP_ROOT"
