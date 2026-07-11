#!/usr/bin/env bash
# Stage the complete, already-built repository update as one local commit.
# It deliberately does not push, tag, or create a hosted release.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

usage() {
    cat <<'EOF'
Usage: scripts/prepare-release.sh [commit message]

Stages the generated repository artifacts, manifests, and changed submodule
pointers, then creates a local commit. It never pushes, creates a tag, or
creates a hosted release.
EOF
}

if [[ ${1:-} == "--help" || ${1:-} == "-h" ]]; then
    usage
    exit 0
fi
if [[ $# -gt 1 ]]; then
    echo "error: pass at most one commit message" >&2
    usage >&2
    exit 2
fi

if [[ -z "$(git status --porcelain --untracked-files=normal)" ]]; then
    echo "error: there is nothing to prepare" >&2
    exit 1
fi

# The update/build scripts are the only writers in this workflow, so stage the
# reviewed worktree exactly as-is. `git diff --cached --check` catches common
# whitespace errors before a local release commit is created.
git add -A
git diff --cached --check

message="${1:-Update package sources and repository artifacts}"
git commit -m "$message"

echo
echo "Local release commit created. Push, tag, and create a hosted release separately."
