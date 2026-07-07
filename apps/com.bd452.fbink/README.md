# com.bd452.fbink

KPM packaging for [FBInk](https://github.com/NiLuJe/FBInk).

Upstream source is a **git submodule** at `vendor/FBInk` (see repo root `.gitmodules`).
Do not copy FBInk into this tree; run `git submodule update --init --recursive` from the
repo root instead.

`./build.sh` cross-compiles the `fbink` CLI for `kindlehf` and `kindlepw2` using the
KindleModding koxtoolchain, stages binaries under `package/bin/` (gitignored), then packs
a `.kpkg` for this repository.

Installed layout on device:

```text
/mnt/us/kmc/kpm/packages/com.bd452.fbink/bin/kindlehf/fbink
/mnt/us/kmc/kpm/packages/com.bd452.fbink/bin/kindlepw2/fbink
```

Other packages should depend on `com.bd452.fbink` and call fbink from that path.
