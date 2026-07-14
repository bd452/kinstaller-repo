#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

shopt -s nullglob
sources=("$REPO_ROOT"/registry/sources/*.json)
if [[ ${#sources[@]} -eq 0 ]]; then
    echo "error: no release descriptors found under registry/sources" >&2
    exit 1
fi

"$REPO_ROOT/scripts/kpm-dev" index "${sources[@]}" \
    --repository-id kinstallerrepo \
    --repository-name "BD452 Kindle Packages" \
    --repository-description "KPM repository for BD452 Kindle software and related homebrew packages" \
    --output "$REPO_ROOT/manifest.json"

echo "Generated $REPO_ROOT/manifest.json from ${#sources[@]} release descriptor(s)."
