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
index.html             # Generated browsing page
scripts/               # Descriptor verification, catalog generation, and Pages staging
```

Generate and validate the catalog with:

```bash
python3 scripts/verify-pinned-artifacts.py
./scripts/build-registry.sh
git diff --exit-code -- manifest.json
python3 index_builder/build_index.py
./scripts/stage-site.sh
```

`scripts/kpm-dev` resolves the pinned `kindle-kpm-devkit` version from
`KPM_DEV`, `PATH`, or a sibling checkout.

Historical descriptors resolve through the immutable
[`legacy-artifacts-v1`](https://github.com/bd452/kinstaller-repo/releases/tag/legacy-artifacts-v1)
release. CI downloads every pinned artifact and verifies its exact byte size and
SHA-256 digest before deploying the catalog.

## Source ownership

This repository intentionally contains no product source, cross-toolchain,
submodules, or `.kpkg` payloads. Those belong to the independent source
repositories:

- [FBInk and Demo](https://github.com/bd452/kindle-fbink-kpm)
- [Ember and Ember Demo](https://github.com/bd452/ember)
- [Kindle Substrate and its demo](https://github.com/bd452/kindle-substrate)
- [Kinstaller](https://github.com/bd452/kinstaller)
- [Shared Kindle KPM devkit](https://github.com/bd452/kindle-kpm-devkit)

Each product repository builds both supported Kindle ABIs and publishes its
`.kpkg` files together with `release-metadata.json`. To update the catalog,
review that descriptor, add it under `registry/sources/`, regenerate
`manifest.json`, and open a pull request. The deployment rejects unreachable or
hash-mismatched artifacts.

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

Package documentation and device instructions live in each source repository.
