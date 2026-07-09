# com.bd452.signalkit

KPM package for the SignalKit library: `libsignalkit.so` (C ABI + FBInk
backend, statically linked) and `signalkit.h`.

Rust sources: [`rust/signalkit`](../../rust/signalkit/). Docs:
[`rust/signalkit/docs/`](../../rust/signalkit/docs/).

No runtime dependencies — FBInk is linked into the `.so`.

## Build

```sh
# macOS / Docker
./scripts/build-in-container.sh apps/com.bd452.signalkit/build.sh

# Linux x86_64 (kox installed)
./apps/com.bd452.signalkit/build.sh
```

Stages `package/lib/{kindlehf,kindlepw2}/libsignalkit.so` and
`package/include/signalkit.h`, then packs a `.kpkg`.
