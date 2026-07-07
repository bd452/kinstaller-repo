#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
KPM_DIR="$REPO_ROOT/.kpm"
KPM_HELPER="$KPM_DIR/kpm-helper.py"
KPM_URL="https://raw.githubusercontent.com/KindleModding/KPM/main/kpm-helper.py"

mkdir -p "$KPM_DIR"

if [[ -f "$KPM_HELPER" ]]; then
    exit 0
fi

echo "==> Downloading kpm-helper.py"
curl -fsSL "$KPM_URL" -o "$KPM_HELPER"
python3 "$REPO_ROOT/scripts/patch-kpm-helper.py" "$KPM_HELPER"
