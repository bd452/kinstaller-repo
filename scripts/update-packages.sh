#!/usr/bin/env bash
# Advance package source submodules, then build and republish affected packages.
# This intentionally leaves committing, tagging, releasing, and pushing to the
# separate release-preparation step and the maintainer.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

usage() {
    cat <<'EOF'
Usage: scripts/update-packages.sh [package-id]

With no package ID, update all package-source submodules and rebuild every
package. With a package ID, update the source submodule owned by that package,
then rebuild it and any packages whose published dependency constraint changes.

Package IDs:
  com.bd452.fbink          (also rebuilds demo and Ember packages)
  com.bd452.demo
  com.bd452.ember          (also rebuilds Ember demo)
  com.bd452.emberdemo
  com.bd452.ksubstrate     (also rebuilds Kindle Substrate demo)
  com.bd452.ksubstratedemo
EOF
}

if [[ ${1:-} == "--help" || ${1:-} == "-h" ]]; then
    usage
    exit 0
fi
if [[ $# -gt 1 ]]; then
    echo "error: pass at most one package ID" >&2
    usage >&2
    exit 2
fi

# A clean starting point makes the resulting update reviewable and prevents an
# upstream checkout from being mixed with unrelated local edits. Ignored build
# outputs are deliberately allowed.
if [[ -n "$(git status --porcelain --untracked-files=normal)" ]]; then
    echo "error: working tree is not clean; commit, stash, or remove changes first" >&2
    exit 1
fi

git submodule update --init --recursive

target="${1:-all}"
update_fbink=false
update_ember=false
update_ksubstrate=false
build_targets=()

case "$target" in
    all)
        update_fbink=true
        update_ember=true
        update_ksubstrate=true
        ;;
    com.bd452.fbink)
        update_fbink=true
        build_targets=(com.bd452.fbink com.bd452.demo com.bd452.ember com.bd452.emberdemo)
        ;;
    com.bd452.demo)
        build_targets=(com.bd452.demo)
        ;;
    com.bd452.ember)
        update_ember=true
        build_targets=(com.bd452.ember com.bd452.emberdemo)
        ;;
    com.bd452.emberdemo)
        update_ember=true
        build_targets=(com.bd452.ember com.bd452.emberdemo)
        ;;
    com.bd452.ksubstrate)
        update_ksubstrate=true
        build_targets=(com.bd452.ksubstrate com.bd452.ksubstratedemo)
        ;;
    com.bd452.ksubstratedemo)
        # The demo links against the runtime library staged by this package.
        build_targets=(com.bd452.ksubstrate com.bd452.ksubstratedemo)
        ;;
    *)
        echo "error: unknown package ID: $target" >&2
        usage >&2
        exit 2
        ;;
esac

if [[ "$update_fbink" == true ]]; then
    echo "==> Updating FBInk to its newest version tag"
    git -C apps/com.bd452.fbink/vendor/FBInk fetch --tags origin
    fbink_tag="$(git -C apps/com.bd452.fbink/vendor/FBInk tag -l 'v[0-9]*' --sort=-version:refname | sed -n '1p')"
    if [[ -z "$fbink_tag" ]]; then
        echo "error: no FBInk version tag matching v<version> was found" >&2
        exit 1
    fi
    git -C apps/com.bd452.fbink/vendor/FBInk checkout --detach "$fbink_tag"
    git -C apps/com.bd452.fbink/vendor/FBInk submodule update --init --recursive
fi

if [[ "$update_ember" == true ]]; then
    echo "==> Updating Ember to its upstream default branch"
    git submodule update --remote -- components/ember
    git -C components/ember submodule update --init --recursive
fi

if [[ "$update_ksubstrate" == true ]]; then
    echo "==> Updating Kindle Substrate to its upstream default branch"
    # Do not use --recursive here: Dobby is deliberately pinned by Kindle
    # Substrate because newer upstream commits do not build with this package.
    git submodule update --remote -- components/kindle-substrate
    git -C components/kindle-substrate submodule update --init --recursive
fi

if [[ "$target" == all ]]; then
    KINSTALLER_SKIP_SUBMODULE_SYNC=1 "$REPO_ROOT/build.sh"
else
    KINSTALLER_SKIP_SUBMODULE_SYNC=1 "$REPO_ROOT/scripts/build-repo.sh" "${build_targets[@]}"
fi

echo
echo "Update complete. Review the diff, then run scripts/prepare-release.sh when ready."
