# App sources for this KPM repository.

Each subdirectory is one package id. Run `./build.sh` at the repo root to build
every app, copy `.kpkg` files into `packages/`, and refresh `manifest.json`.

On macOS, prefer:

```sh
docker build --platform linux/amd64 -t kinstaller-build .
./scripts/build-in-container.sh
```

Third-party source lives in git submodules rather than being copied into this
repository. Kindle Substrate is also a first-party component submodule and
contains its own nested Dobby dependency. Use `git clone --recurse-submodules`
or `git submodule update --init --recursive` before building.

SignalKit packages:

- [`com.bd452.signalkit`](com.bd452.signalkit/) — `libsignalkit.so` + header
- [`com.bd452.signalkitdemo`](com.bd452.signalkitdemo/) — interactive demo

Kindle Substrate packages:

- [`com.bd452.ksubstrate`](../components/kindle-substrate/apps/com.bd452.ksubstrate/) — runtime/control package from the Kindle Substrate submodule
- [`com.bd452.ksubstratedemo`](../components/kindle-substrate/apps/com.bd452.ksubstratedemo/) — self-contained runtime demo from the Kindle Substrate submodule

GitHub Pages serves the committed `packages/` tree directly. Symlinks are not
used because GitHub Pages does not reliably follow them.
