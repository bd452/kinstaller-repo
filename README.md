# BD452 Kindle Packages

Static [KPM](https://kindlemodding.org/kindle-dev/kpm/) catalog for BD452
Kindle software and related homebrew packages.

- **Manifest:** https://bd452.github.io/kinstaller-repo/manifest.json
- **Browse:** https://bd452.github.io/kinstaller-repo/

## Add on your Kindle

```text
;kpm add-repo https://bd452.github.io/kinstaller-repo/manifest.json
```

## Architecture

Package source repositories build, test, and release their own immutable
`.kpkg` files with `kindle-kpm-devkit`. This repository pins their
`release-metadata.json` descriptors, validates the combined dependency graph,
and publishes the generated KPM manifest and browsing UI.

See [the artifact-first architecture](docs/artifact-first-architecture.md) for
the ownership and migration contract.

## Repository layout

```text
registry/sources/     # Reviewed, immutable release descriptors
manifest.json         # Generated KPM index
index_builder/        # Static browsing UI
packages/             # Transitional Pages-hosted artifacts
apps/                 # Legacy local package recipes
components/           # Legacy first-party source submodules
scripts/              # Registry generation and compatibility tooling
```

Generate and validate the catalog with:

```bash
./scripts/build-registry.sh
python3 index_builder/build_index.py
./scripts/stage-site.sh
```

`scripts/kpm-dev` resolves the pinned `kindle-kpm-devkit` version from
`KPM_DEV`, `PATH`, or a sibling checkout.

GitHub Pages serves the committed `packages/` tree. Symlinks are not used because
GitHub Pages does not reliably follow them.

## Legacy source builds

The source build remains available while FBInk, Demo, Ember, and Kindle
Substrate complete independent artifact releases. It is no longer the target
architecture for new packages.

Third-party native engines are **not** copied into this repository. They are
[git submodules](https://git-scm.com/book/en/v2/Git-Tools-Submodules):

- `apps/com.bd452.fbink/vendor/FBInk` — FBInk (pinned release tag)
- `components/ember` — Ember UI framework and package sources
- `components/ember/apps/com.bd452.fbink/vendor/FBInk` — nested FBInk dependency
- `components/kindle-substrate` — Kindle Substrate runtime, toolchain, and package sources
- `components/kindle-substrate/apps/com.bd452.ksubstrate/vendor/Dobby` — nested Dobby inline-hook engine

Only the submodule commit pointers are stored here; clone with:

```bash
git clone --recurse-submodules https://github.com/bd452/kinstaller-repo.git
# or after a plain clone:
git submodule update --init --recursive
```

Build everything and refresh the published repository tree:

```bash
./build.sh
```

`com.bd452.fbink` requires the [KindleModding koxtoolchain](https://github.com/KindleModding/koxtoolchain) on Linux x86_64:

```bash
./scripts/setup-koxtoolchain.sh
./build.sh
```

On macOS, use the repo Docker image (Ubuntu + Rust + koxtoolchain + clang):

```bash
./scripts/build-in-container.sh          # runs ./build.sh in the container
```

The transitional source-build Dockerfile inherits the pinned
`ghcr.io/bd452/kindle-kpm-build:v0.1.0@sha256:c7bd7e4041717bb16765b97d6fe4f578f40d144fa3628fcad81271e22f18a69b`
image from `kindle-kpm-devkit`.

The helper builds or refreshes `kinstaller-repo-build:kpm-devkit-0.1.0` when needed and mounts
the repo with `CARGO_TARGET_DIR=components/ember/target-kindle` for Ember builds.

Or run on a Linux host with the toolchain installed (`./scripts/setup-koxtoolchain.sh`).
Ember / demo packaging details: [`components/ember/ember/docs/building.md`](components/ember/ember/docs/building.md),
[`components/ember/apps/com.bd452.emberdemo/README.md`](components/ember/apps/com.bd452.emberdemo/README.md).

Then commit `manifest.json`, `packages/`, and any updated app manifests, and push.
GitHub Actions rebuilds the web index and publishes to GitHub Pages.

## Updating package sources and preparing a release

Use the update workflow to advance source submodules and rebuild the generated
package repository. It never commits, pushes, tags, or creates a hosted release.
Start from a clean worktree:

```bash
# On Linux x86_64 with the toolchain installed:
./scripts/update-packages.sh

# On macOS:
./scripts/build-in-container.sh ./scripts/update-packages.sh
```

To update a package source and rebuild only its affected package set, pass its
published package ID. For example, updating FBInk also rebuilds the demo and
Ember packages that use it; updating Kindle Substrate also rebuilds its demo:

```bash
./scripts/update-packages.sh com.bd452.fbink
./scripts/update-packages.sh com.bd452.ksubstrate
```

Packages without an independently updateable source submodule are simply
rebuilt; the Kindle Substrate demo also rebuilds its runtime prerequisite.

FBInk advances to its newest `v<major>.<minor>.<patch>` release tag. Kindle
Substrate advances to its upstream default branch; its nested Dobby dependency
remains at the commit pinned by Kindle Substrate because newer Dobby revisions
are not compatible with the package build.

After reviewing the resulting diff, create a local release commit without any
external publication side effects:

```bash
./scripts/prepare-release.sh "Update package sources"
```

Push that commit, create a tag, and create a hosted release separately when
you are ready to publish.

## Installing packages

```text
;kpm install com.bd452.fbink
;kpm install com.bd452.demo
;kpm install com.bd452.emberdemo
```

Dependent apps resolve tools under `/mnt/us/kmc/kpm/packages/<dependency-id>/`.
For example, the demo app calls FBInk at:

```text
/mnt/us/kmc/kpm/packages/com.bd452.fbink/bin/<platform>/fbink
```

Kindle Substrate architecture and package contracts are documented in
[`components/kindle-substrate/docs/kindle-substrate.md`](components/kindle-substrate/docs/kindle-substrate.md).
The root build compiles and publishes the runtime package plus its
self-contained demo after the existing FBInk and Ember packages.
