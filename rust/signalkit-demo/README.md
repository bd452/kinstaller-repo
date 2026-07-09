# signalkit-demo

On-device / host demo for [`signalkit`](../signalkit/).

- **Host** (default): mounts the demo screen, renders one frame with
  `MockRenderer`, exits. No FBInk or cross toolchain.
- **Device** (`--features fbink`): `FbinkRenderer` + touch `App::run`.

KPM packaging (binaries per platform, launcher, `.kpkg`) lives in
[`apps/com.bd452.signalkitdemo`](../../apps/com.bd452.signalkitdemo/) — see that
README for Docker/Linux build and deploy.

Library docs: [`signalkit/docs/`](../signalkit/docs/), especially
[building.md](../signalkit/docs/building.md) and [usage.md](../signalkit/docs/usage.md).

```sh
# Host smoke
cargo run -p signalkit-demo

# Device (from repo root, via container helper on macOS)
./scripts/build-in-container.sh apps/com.bd452.signalkitdemo/build.sh
```
