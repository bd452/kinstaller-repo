#!/usr/bin/env bash
set -euo pipefail

APP_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$APP_ROOT/../.." && pwd)"

python3 "$APP_ROOT/scripts/make-icon.py"
"$REPO_ROOT/scripts/pack-app.sh" "$APP_ROOT"
