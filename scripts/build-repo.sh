#!/usr/bin/env bash
# Build all apps and sync packages/ + manifest.json for GitHub Pages.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

if [[ -f .gitmodules ]]; then
    git submodule update --init --recursive
fi

"$REPO_ROOT/scripts/fetch-kpm-helper.sh"
KPM_HELPER="$REPO_ROOT/.kpm/kpm-helper.py"

APP_ORDER=(
    apps/com.bd452.fbink
    apps/com.bd452.demo
)

echo "==> Building apps"
for app_path in "${APP_ORDER[@]}"; do
    if [[ ! -x "$app_path/build.sh" ]]; then
        echo "error: missing build.sh in $app_path" >&2
        exit 1
    fi
    echo
    "$app_path/build.sh"
done

echo
echo "==> Resetting repository package index"
python3 "$REPO_ROOT/scripts/reset-repo-packages.py"

echo "==> Publishing .kpkg artifacts into packages/"
for app_path in "${APP_ORDER[@]}"; do
    kpkg="$(find "$app_path/dist" -maxdepth 1 -name '*.kpkg' -print -quit)"
    if [[ -z "$kpkg" ]]; then
        echo "error: no .kpkg found in $app_path/dist" >&2
        exit 1
    fi
    python3 "$KPM_HELPER" repo add "$REPO_ROOT/manifest.json" "$kpkg"
done

echo
echo "Repository manifest and packages/ are up to date."
