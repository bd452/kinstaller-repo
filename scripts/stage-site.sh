#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SITE_DIR="${1:-$REPO_ROOT/site}"

rm -rf "$SITE_DIR"
mkdir -p "$SITE_DIR"

cp "$REPO_ROOT/manifest.json" "$SITE_DIR/manifest.json"
cp "$REPO_ROOT/index.html" "$SITE_DIR/index.html"

echo "Staged Pages site at $SITE_DIR"
