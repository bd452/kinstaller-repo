# App sources for this KPM repository.

Each subdirectory is one package id. Run `./build.sh` at the repo root to build
every app, copy `.kpkg` files into `packages/`, and refresh `manifest.json`.

On macOS, prefer:

```sh
docker build --platform linux/amd64 -t kinstaller-build .
./scripts/build-in-container.sh
```

Third-party source lives in git submodules under `vendor/` — not copied into
this repository (FBInk for fbink/signalkit, Dobby for ksubstrate). Use
`git clone --recurse-submodules` or `git submodule update --init --recursive`
before building.

SignalKit packages:

- [`com.bd452.signalkit`](com.bd452.signalkit/) — `libsignalkit.so` + header
- [`com.bd452.signalkitdemo`](com.bd452.signalkitdemo/) — interactive demo

Kindle Substrate packages:

- [`com.bd452.ksubstrate`](com.bd452.ksubstrate/) — runtime/control package
- [`com.bd452.ksubstratedemo`](com.bd452.ksubstratedemo/) — self-contained runtime demo

GitHub Pages serves the committed `packages/` tree directly. Symlinks are not
used because GitHub Pages does not reliably follow them.
