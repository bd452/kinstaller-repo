#!/usr/bin/env bash
# Resolve KindleModding koxtoolchain compiler prefixes for FBInk builds.
set -euo pipefail

KOX_BASE="${KOXTOOLCHAIN_ROOT:-$HOME/x-tools}"
export KOX_BASE

kox_prefix() {
    case "$1" in
        kindlehf) echo "arm-kindlehf-linux-gnueabihf" ;;
        kindlepw2) echo "arm-kindlepw2-linux-gnueabi" ;;
        *)
            echo "unknown platform: $1" >&2
            return 1
            ;;
    esac
}

kox_gcc() {
    local prefix
    prefix="$(kox_prefix "$1")"
    echo "$KOX_BASE/x-tools/${prefix}/bin/${prefix}-gcc"
}

require_kox() {
    local platform missing=0
    for platform in kindlehf kindlepw2; do
        if [[ ! -x "$(kox_gcc "$platform")" ]]; then
            echo "error: missing koxtoolchain for $platform" >&2
            echo "  expected: $(kox_gcc "$platform")" >&2
            missing=1
        fi
    done
    if [[ "$missing" -ne 0 ]]; then
        echo "Install with: ./scripts/setup-koxtoolchain.sh" >&2
        return 1
    fi
}
