# Artifact-first repository architecture

This repository is the KPM catalog and deployment boundary. Package source,
cross-compilation, device tests, and release production belong to each source
repository.

## Data flow

1. A source repository builds and tests its package with `kindle-kpm-devkit`.
2. A tag publishes immutable `.kpkg` artifacts and `release-metadata.json`.
3. This repository pins reviewed release descriptors under `registry/sources/`.
4. `scripts/build-registry.sh` validates the combined dependency graph and
   generates `manifest.json` without rebuilding or copying source projects.
5. GitHub Pages publishes the manifest and browsing UI. Artifact URLs may point
   to GitHub Releases, object storage, or transitional files already hosted by
   this Pages site.

The existing `apps/`, `components/`, and `scripts/build-repo.sh` flow remains a
temporary compatibility path while each package source gains an independent
release. New first-party components must use release descriptors instead of
source submodules.

## Ownership

- Source repositories own compilation, tests, package contents, tags, and
  immutable artifacts.
- `kindle-kpm-devkit` owns manifest validation, deterministic packing, release
  descriptor generation, and registry generation.
- This repository owns reviewed artifact pins, catalog metadata, the generated
  KPM manifest, and Pages deployment.

`manifest.json` is generated. Do not hand-edit its package entries after the
descriptor migration is complete.
