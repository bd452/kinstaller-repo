#!/usr/bin/env bash
# Build apps and sync packages/ + manifest.json for GitHub Pages.
#
# With no arguments, rebuild the complete repository.  With one or more package
# IDs, rebuild and republish only those packages.  Targeted builds preserve the
# rest of the repository index and are useful to the update workflow.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

usage() {
    cat <<'EOF'
Usage: scripts/build-repo.sh [package-id ...]

Build every package when no package IDs are supplied.  Otherwise build and
republish only the named package IDs.
EOF
}

if [[ ${1:-} == "--help" || ${1:-} == "-h" ]]; then
    usage
    exit 0
fi

if [[ -f .gitmodules && ${KINSTALLER_SKIP_SUBMODULE_SYNC:-0} != 1 ]]; then
    git submodule update --init --recursive
fi

"$REPO_ROOT/scripts/fetch-kpm-helper.sh"
KPM_HELPER="$REPO_ROOT/.kpm/kpm-helper.py"

APP_ORDER=(
    apps/com.bd452.fbink
    apps/com.bd452.demo
    apps/com.bd452.signalkit
    apps/com.bd452.signalkitdemo
    components/kindle-substrate/apps/com.bd452.ksubstrate
    components/kindle-substrate/apps/com.bd452.ksubstratedemo
)

declare -A APP_BY_ID=(
    [com.bd452.fbink]=apps/com.bd452.fbink
    [com.bd452.demo]=apps/com.bd452.demo
    [com.bd452.signalkit]=apps/com.bd452.signalkit
    [com.bd452.signalkitdemo]=apps/com.bd452.signalkitdemo
    [com.bd452.ksubstrate]=components/kindle-substrate/apps/com.bd452.ksubstrate
    [com.bd452.ksubstratedemo]=components/kindle-substrate/apps/com.bd452.ksubstratedemo
)

TARGET_APPS=()
if [[ $# -eq 0 ]]; then
    TARGET_APPS=("${APP_ORDER[@]}")
else
    declare -A SEEN_IDS=()
    for package_id in "$@"; do
        app_path="${APP_BY_ID[$package_id]:-}"
        if [[ -z "$app_path" ]]; then
            echo "error: unknown package ID: $package_id" >&2
            usage >&2
            exit 2
        fi
        if [[ -z "${SEEN_IDS[$package_id]:-}" ]]; then
            TARGET_APPS+=("$app_path")
            SEEN_IDS[$package_id]=1
        fi
    done
fi

echo "==> Building apps"
for app_path in "${TARGET_APPS[@]}"; do
    if [[ ! -x "$app_path/build.sh" ]]; then
        echo "error: missing build.sh in $app_path" >&2
        exit 1
    fi
    echo
    "$app_path/build.sh"
done

if [[ $# -eq 0 ]]; then
    echo
    echo "==> Resetting repository package index"
    python3 "$REPO_ROOT/scripts/reset-repo-packages.py"
fi

echo "==> Publishing .kpkg artifacts into packages/"
for app_path in "${TARGET_APPS[@]}"; do
    kpkg="$(find "$app_path/dist" -maxdepth 1 -name '*.kpkg' -print -quit)"
    if [[ -z "$kpkg" ]]; then
        echo "error: no .kpkg found in $app_path/dist" >&2
        exit 1
    fi
    package_id="$(python3 -c 'import json, sys; print(json.load(open(sys.argv[1], encoding="utf-8"))["id"])' "$app_path/package/manifest.json")"
    # kpm-helper replaces the manifest entry, but old artifact filenames would
    # otherwise remain beside the newly built package after a version bump.
    rm -rf "$REPO_ROOT/packages/$package_id"
    python3 "$KPM_HELPER" repo add "$REPO_ROOT/manifest.json" "$kpkg"
done

echo
echo "Repository manifest and packages/ are up to date."
