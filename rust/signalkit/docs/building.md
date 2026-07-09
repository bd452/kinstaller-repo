# Building and developing SignalKit

This guide is written so a fresh checkout (or a new chat context) can build,
test, cross-compile, and deploy **without reading the rest of the repo**.

## What you can do where

| Task | macOS | Linux x86_64 | Windows |
|------|-------|--------------|---------|
| Host tests / demo stub | Yes | Yes | Yes (WSL2 recommended) |
| Cross-compile for Kindle | Via Linux container only | Native | Via Linux container / WSL2 |
| Install koxtoolchain | No (Linux binaries) | Yes | Via Linux |
| Run on Kindle | Deploy over USB/SSH/KPM | Same | Same |

**Hard rule:** the KindleModding koxtoolchain ships **Linux x86_64** host
binaries. On macOS/ARM hosts you must cross-build inside a **linux/amd64**
container (Docker Desktop, OrbStack, Podman, etc.), not with the macOS
toolchain.

## Repository layout (Rust-related)

```
<repo>/
  Dockerfile                # kinstaller-build image (Ubuntu + Rust + kox + clang)
  rust/
    Cargo.toml              # workspace: fbink-sys, signalkit, signalkit-demo
    Cargo.lock              # committed
    .cargo/config.toml      # linker names for the two ARM triples
    signalkit/              # library
    signalkit-demo/         # demo binary (+ README)
    fbink-sys/              # builds + binds vendored FBInk
  apps/com.bd452.fbink/vendor/FBInk/   # git submodule (required for fbink)
  apps/com.bd452.signalkit/            # KPM package → libsignalkit.so + header
  apps/com.bd452.signalkitdemo/        # KPM package → demo (+ README)
  scripts/koxtoolchain.sh              # platform → prefix / Rust triple / PATH
  scripts/setup-koxtoolchain.sh        # download toolchains (Linux only)
  scripts/build-in-container.sh        # macOS/dev: run builds in kinstaller-build
  scripts/build-repo.sh                # ./build.sh entry → all packages
```

## Prerequisites

### All platforms (host work)

- Git
- Rust stable (`rustup`):
  ```sh
  rustup toolchain install stable
  # optional until you cross-compile:
  rustup target add armv7-unknown-linux-gnueabihf armv7-unknown-linux-gnueabi
  ```
- No FBInk, no koxtoolchain, and no `fbink` feature needed for host tests.

### Linux x86_64 (cross-compile / package builds)

- Packages: `build-essential`, `curl`, `python3`, `make`
- For `fbink-sys` bindgen: `clang`, `libclang-dev`, `llvm-dev`
  ```sh
  # Debian/Ubuntu
  sudo apt-get update
  sudo apt-get install -y build-essential curl python3 make \
    clang llvm-dev libclang-dev
  ```
- koxtoolchain (see below)

### macOS (cross-compile)

- Docker Desktop, OrbStack, or Podman
- Build/run via repo [`Dockerfile`](../../../Dockerfile) +
  [`scripts/build-in-container.sh`](../../../scripts/build-in-container.sh)
  (see §5). Host Rust is enough for tests only.

## Clone

FBInk is a **git submodule**. Without it, any `--features fbink` build fails.

```sh
git clone --recurse-submodules https://github.com/bd452/kinstaller-repo.git
cd kinstaller-repo

# If you already cloned without submodules:
git submodule update --init --recursive
```

Confirm:

```sh
test -f apps/com.bd452.fbink/vendor/FBInk/Makefile && echo "FBInk OK"
```

## 1. Host development (any OS)

From the repo:

```sh
cd rust
cargo test -p signalkit
cargo test -p signalkit --features capi
cargo run -p signalkit-demo
```

What this does:

- Builds the reactive core + mock renderer (no FBInk).
- Demo without `fbink` mounts the UI, renders **one** frame to `MockRenderer`,
  prints a line, and exits.

Useful crate features:

| Feature | On by default? | Purpose |
|---------|----------------|---------|
| *(none)* | — | Core + mock renderer |
| `fbink` | no | FBInk backend, touch, `App::run` (cross only in practice) |
| `capi` | no | C ABI in `ffi.rs` / `libsignalkit.so` |

## 2. Cross-compilation overview

### Platforms and triples

| KPM platform | Rust target | kox prefix | Float ABI |
|--------------|-------------|------------|-----------|
| `kindlehf` | `armv7-unknown-linux-gnueabihf` | `arm-kindlehf-linux-gnueabihf` | hard |
| `kindlepw2` | `armv7-unknown-linux-gnueabi` | `arm-kindlepw2-linux-gnueabi` | soft |

Helpers live in `scripts/koxtoolchain.sh`:

| Function | Returns |
|----------|---------|
| `kox_prefix <plat>` | Compiler prefix string |
| `kox_rust_target <plat>` | Rust triple |
| `kox_tool_bin <plat>` | `$KOX_BASE/x-tools/<prefix>/bin` (prepend to `PATH`) |
| `kox_gcc <plat>` | Full path to `*-gcc` |
| `require_kox` | Exits nonzero if either toolchain is missing |

Default install root:

```text
KOXTOOLCHAIN_ROOT  →  defaults to $HOME/x-tools
Expected gcc:      $KOXTOOLCHAIN_ROOT/x-tools/<prefix>/bin/<prefix>-gcc
```

`rust/.cargo/config.toml` names the linkers as bare tool names
(`arm-kindlehf-linux-gnueabihf-gcc`, …). Package build scripts also export
the absolute `CARGO_TARGET_<TRIPLE_WITH_UNDERSCORES>_LINKER` value for each
platform, because Cargo does not always discover `rust/.cargo/config.toml`
when invoked from the repo root with `--manifest-path`.

### Environment variables the Rust build expects

| Variable | Required when | Meaning |
|----------|---------------|---------|
| `CROSS_TC` | `fbink` feature | Prefix passed to FBInk’s Makefile (e.g. `arm-kindlehf-linux-gnueabihf`) |
| `PATH` | cross build | Must include `kox_tool_bin` so `$CROSS_TC-gcc` resolves |
| `CARGO_TARGET_…_LINKER` | cross build | Absolute kox gcc path used by Cargo/rustc for the ARM target |
| `KOXTOOLCHAIN_ROOT` | optional | Override toolchain root (default `$HOME/x-tools`) |
| `CARGO_TARGET_DIR` | optional | e.g. `rust/target-kindle` to keep host and cross artifacts separate |

### What `fbink-sys` does at build time

When `--features fbink` is enabled, `rust/fbink-sys/build.rs`:

1. Asserts the FBInk submodule exists.
2. Runs `make clean` then `make pic KINDLE=true MINIMAL=1 DRAW=1 BITMAP=1 CROSS_TC=…`
   in `apps/com.bd452.fbink/vendor/FBInk`.
3. Copies `Release/libfbink.a` into Cargo's `OUT_DIR` and links that stable
   copy (+ `libm`).
4. Runs **bindgen** on `fbink.h` with `--target=<triple>` and the cross
   sysroot (needs libclang on the **build** machine).

Important details:

- FBInk’s `Release/` dir is **shared across targets** — always clean between
  `kindlehf` and `kindlepw2` (the package `build.sh` scripts do this).
- Cargo links against the archive copy in `OUT_DIR`, not the shared
  `Release/` path, so a later FBInk clean cannot remove the archive before
  `rustc` consumes it.
- Cargo sets `DEBUG=true/false`; FBInk’s Makefile treats **presence** of
  `DEBUG` as “use Debug/”. `build.rs` unsets `DEBUG` / `OPT_LEVEL` so the
  archive lands in `Release/`.
- Use `make pic` (PIC static lib), not plain `static`, so `libsignalkit.so`
  (cdylib) can link it (`-fPIC`).

## 3. Install koxtoolchain (Linux x86_64)

On a real Linux x86_64 host or inside a linux/amd64 container:

```sh
./scripts/setup-koxtoolchain.sh
```

This downloads KindleModding release tarballs into
`$KOXTOOLCHAIN_ROOT/x-tools` (default `~/x-tools`):

- `kindlehf.tar.gz`
- `kindlepw2.tar.gz`

Verify:

```sh
source scripts/koxtoolchain.sh
require_kox
"$(kox_gcc kindlehf)" --version
"$(kox_gcc kindlepw2)" --version
```

On macOS this script **exits immediately** — use a container (next section).

Also install Rust targets on that Linux environment:

```sh
rustup target add armv7-unknown-linux-gnueabihf armv7-unknown-linux-gnueabi
```

## 4. Cross-build on Linux (native)

One-time setup:

```sh
git submodule update --init --recursive
./scripts/setup-koxtoolchain.sh
rustup toolchain install stable
rustup target add armv7-unknown-linux-gnueabihf armv7-unknown-linux-gnueabi
sudo apt-get install -y clang llvm-dev libclang-dev   # if not already
```

### Preferred: package scripts

These stage artifacts under `apps/.../package/` and pack `.kpkg` files:

```sh
# Library: libsignalkit.so (capi+fbink) for both platforms + header
apps/com.bd452.signalkit/build.sh

# Demo binary for both platforms + icon + .kpkg
apps/com.bd452.signalkitdemo/build.sh

# Entire repo (all apps + manifest refresh)
./build.sh
```

Outputs:

| Script | Artifacts |
|--------|-----------|
| signalkit `build.sh` | `apps/com.bd452.signalkit/package/lib/{kindlehf,kindlepw2}/libsignalkit.so`, `package/include/signalkit.h` |
| demo `build.sh` | `apps/com.bd452.signalkitdemo/package/bin/{kindlehf,kindlepw2}/signalkit-demo` |
| cargo (default) | `rust/target/<triple>/release/…` |

### Manual cargo (kindlehf example)

```sh
cd /path/to/kinstaller-repo
source scripts/koxtoolchain.sh
require_kox

platform=kindlehf
cross_tc="$(kox_prefix "$platform")"
tool_bin="$(kox_tool_bin "$platform")"
rust_target="$(kox_rust_target "$platform")"

export CROSS_TC="$cross_tc"
export PATH="$tool_bin:$PATH"
# Absolute linker avoids PATH surprises in some containers:
export CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER="$tool_bin/${cross_tc}-gcc"

# Optional: keep cross artifacts out of rust/target
export CARGO_TARGET_DIR="$PWD/rust/target-kindle"

cargo build --manifest-path rust/Cargo.toml \
  -p signalkit-demo --release --features fbink --target "$rust_target"

# Binary:
#   $CARGO_TARGET_DIR/$rust_target/release/signalkit-demo
# or rust/target/$rust_target/release/signalkit-demo
```

Library with C ABI:

```sh
cargo build --manifest-path rust/Cargo.toml \
  -p signalkit --release --features capi,fbink --target "$rust_target"
# → .../release/libsignalkit.so
```

For **kindlepw2**, same pattern with `platform=kindlepw2` and:

```sh
export CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABI_LINKER="$tool_bin/${cross_tc}-gcc"
```

Clean FBInk between platform flips:

```sh
make -C apps/com.bd452.fbink/vendor/FBInk clean
```

## 5. Cross-build with Docker (macOS and recommended for agents)

This is the path we use day-to-day on macOS. The repo ships a
[`Dockerfile`](../../../Dockerfile) that produces the **`kinstaller-build`**
image: Ubuntu 24.04, build tools, **clang/libclang** (bindgen), Rust stable +
both ARM targets, and both kox toolchains under `/opt/x-tools`.

### Build the image (once)

From the **repo root** (always force amd64 on Apple Silicon):

```sh
docker build --platform linux/amd64 -t kinstaller-build .
```

Takes several minutes the first time (apt + rustup + toolchain tarballs).
Reuse `kinstaller-build:latest` afterward.

The helper also checks that the local image contains the expected build
prerequisites (`clang`, `libclang`, and `rustfmt`). If an older
`kinstaller-build:latest` is missing them, it rebuilds from the repo
`Dockerfile` before running your command.

Image contract (do not guess paths):

| Item | Value |
|------|--------|
| Tag | `kinstaller-build:latest` (override with `KINSTALLER_BUILD_IMAGE`) |
| Platform | `linux/amd64` |
| `KOXTOOLCHAIN_ROOT` | `/opt/x-tools` |
| gcc (kindlehf) | `/opt/x-tools/x-tools/arm-kindlehf-linux-gnueabihf/bin/arm-kindlehf-linux-gnueabihf-gcc` |
| Working dir in helper | `/repo` (repo bind-mount) |

The tarballs extract an `x-tools/` directory *into* `KOXTOOLCHAIN_ROOT`, and
`scripts/koxtoolchain.sh` joins `$KOXTOOLCHAIN_ROOT/x-tools/<prefix>/…`. So the
root must be `/opt/x-tools`, **not** `/opt/x-tools/x-tools`.

### Helper script (preferred)

[`scripts/build-in-container.sh`](../../../scripts/build-in-container.sh)
builds the image if missing, mounts the repo, sets `KOXTOOLCHAIN_ROOT` and
`CARGO_TARGET_DIR=/repo/rust/target-kindle`, and runs your command:

```sh
# Full repo package build
./scripts/build-in-container.sh

# Demo KPM package only (both platforms)
./scripts/build-in-container.sh apps/com.bd452.signalkitdemo/build.sh

# Library package only
./scripts/build-in-container.sh apps/com.bd452.signalkit/build.sh

# Interactive shell inside the image
./scripts/build-in-container.sh bash

# One-off kindlehf demo binary (cargo only)
./scripts/build-in-container.sh bash -lc '
source scripts/koxtoolchain.sh
platform=kindlehf
export CROSS_TC="$(kox_prefix "$platform")"
export PATH="$(kox_tool_bin "$platform"):$PATH"
export CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER="$(kox_tool_bin "$platform")/${CROSS_TC}-gcc"
cargo build --manifest-path rust/Cargo.toml \
  -p signalkit-demo --release --features fbink \
  --target "$(kox_rust_target "$platform")"
'
```

Artifacts:

- Cargo output: `rust/target-kindle/<triple>/release/` on the host (warm
  across runs; the helper sets `CARGO_TARGET_DIR`)
- Package scripts also stage into `apps/.../package/bin|lib/...` and honor
  `CARGO_TARGET_DIR` when copying the built binary/`.so`

### Equivalent raw `docker run`

What the helper does after the image check:

```sh
docker run --rm --platform linux/amd64 \
  -v "$PWD":/repo \
  -e KOXTOOLCHAIN_ROOT=/opt/x-tools \
  -e CARGO_TARGET_DIR=/repo/rust/target-kindle \
  -w /repo \
  kinstaller-build:latest \
  apps/com.bd452.signalkitdemo/build.sh
```

### OrbStack / Podman

Same commands: OrbStack provides a `docker` CLI; Podman often does too.
Always pass `--platform linux/amd64`.

### Fallback — stock Ubuntu (no local image)

Only if you cannot use the repo Dockerfile. You must install clang, Rust,
and kox yourself inside the container (slower, easy to get wrong). Prefer
§5 image + helper.

## 6. Deploying to a Kindle

### Via KPM (proper)

1. Build packages (`./build.sh` or the app `build.sh` scripts).
2. Install the `.kpkg` from this repo’s published feed, or copy the artifact
   from `packages/` / the app `package/` output onto the device and install
   with KPM.
3. Launch from KUAL / the package launcher (`launch.sh` → `app.sh`).

`app.sh` picks `kindlehf` vs `kindlepw2` by checking for
`/lib/ld-linux-armhf.so.3`.

`launch.sh` disables pillow while running and restores home via `appmgrd` on
exit.

### Quick SSH copy (dev loop)

USB network / Wi‑Fi SSH as `root` (jailbroken device). BusyBox `sleep` often
**rejects fractional seconds** — use whole seconds.

Preferred helper:

```sh
# Builds should happen first.
./scripts/build-in-container.sh apps/com.bd452.signalkitdemo/build.sh

# If you know the IP:
KINDLE=root@192.168.1.231 KINDLE_PASSWORD=kindle \
  ./scripts/run-signalkit-demo-on-kindle.sh

# If you do not know the IP, the helper tries SSH against hosts already in the
# local ARP table. Wake the Kindle and ping/browse it first if discovery misses.
KINDLE_PASSWORD=kindle ./scripts/run-signalkit-demo-on-kindle.sh
```

Manual equivalent:

```sh
# Host: path to the kindlehf binary (adjust if you used CARGO_TARGET_DIR)
BIN=rust/target-kindle/armv7-unknown-linux-gnueabihf/release/signalkit-demo
# or: apps/com.bd452.signalkitdemo/package/bin/kindlehf/signalkit-demo

KINDLE=root@192.168.1.231   # your device

ssh "$KINDLE" 'killall signalkit-demo 2>/dev/null || true; sleep 1'
scp "$BIN" "$KINDLE:/mnt/us/signalkit-demo"
ssh "$KINDLE" '
lipc-set-prop com.lab126.pillow disableEnablePillow disable 2>/dev/null || true
cd /mnt/us
nohup ./signalkit-demo >/mnt/us/signalkit-demo.out 2>/mnt/us/signalkit-demo.err &
sleep 2
ps | grep signalkit | grep -v grep
cat /mnt/us/signalkit-demo.err
'
```

If `scp` fails with “Failure” on the dest file, the binary is still running —
`killall` first.

Optional password helper on the host: `sshpass -p '…' ssh …` / `scp …`.

Touch overrides (if taps miss):

```sh
export SIGNALKIT_TOUCH_DEV=/dev/input/event2
export SIGNALKIT_TOUCH_SWAP=1
export SIGNALKIT_TOUCH_INVERT_X=1
export SIGNALKIT_TOUCH_INVERT_Y=1
export SIGNALKIT_TOUCH_RAW=WxH
```

## 7. Regenerating the C header

Committed header: `rust/signalkit/include/signalkit.h`.

```sh
cd rust
cargo install cbindgen --locked   # once
cbindgen --crate signalkit --config signalkit/cbindgen.toml \
  --output signalkit/include/signalkit.h
```

Requires the `capi` types in `ffi.rs`. Do not hand-edit the header. Commit the
regenerated file when the ABI changes.

## 8. CI reference

`.github/workflows/publish.yml` on `ubuntu-latest`:

1. Checkout with `submodules: recursive`
2. `./scripts/setup-koxtoolchain.sh`
3. `rustup` + both ARM targets
4. `apt` install clang / libclang
5. `cargo test` (host, with and without `capi`)
6. `./build.sh` (full package build)

Match that environment when debugging “works on my Mac host tests but CI
cross fails.”

## 9. Release profile

Workspace `rust/Cargo.toml`:

- `opt-level = "z"`, LTO, `codegen-units = 1`, `strip = true`
- `panic = "unwind"` — required for FFI `catch_unwind`

## 10. Troubleshooting

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| `FBInk source missing` | Submodule not initialized | `git submodule update --init --recursive` |
| `missing koxtoolchain` | Not installed / wrong `KOXTOOLCHAIN_ROOT` | Run `setup-koxtoolchain.sh` on Linux; check `kox_gcc` path |
| `arm-kindlehf-…-gcc: not found` | Tool bin not on `PATH` | `export PATH="$(kox_tool_bin kindlehf):$PATH"` or set `CARGO_TARGET_…_LINKER` |
| bindgen / libclang errors | No clang on build host | Install `clang` `libclang-dev` `llvm-dev` |
| Link error about non-PIC / `libfbink.a` | Stale non-PIC archive | `make -C …/FBInk clean` and rebuild with `fbink-sys` (`make pic`) |
| `could not find native static library fbink` | Old `fbink-sys` or Cargo linked FBInk's shared `Release/` dir after a clean | Rebuild current tree; `fbink-sys` now copies `libfbink.a` into `OUT_DIR` before linking |
| Wrong FBInk `Debug/` vs `Release/` | `DEBUG` env from Cargo | Already scrubbed in `build.rs`; if you invoke `make` by hand, unset `DEBUG` |
| Soft/hard float mix | Built hf objects then linked pw2 (or reverse) without clean | Clean FBInk + `cargo clean -p fbink-sys` between platforms |
| `undefined reference to getauxval` on `kindlepw2` | Old soft-float libc lacks a symbol current Rust std references | Current `fbink-sys` provides a target-scoped fallback shim for `armv7-unknown-linux-gnueabi`; rebuild current tree |
| `scp: dest open … Failure` | Binary still running on device | `killall signalkit-demo` then retry |
| `sleep: invalid number '0.5'` | BusyBox sleep | Use `sleep 1` |
| Host `cargo build --features fbink` | Needs cross linker + FBInk Makefile | Only do this with kox env; prefer package `build.sh` |
| `docker build` slow / wrong arch on Mac | Missing `--platform linux/amd64` | Always pass it on Apple Silicon |
| Image missing clang/libclang/rustfmt | Old custom image | Use `scripts/build-in-container.sh`; it validates and rebuilds stale images |
| `docker_tty[@]: unbound variable` | Old container helper under non-TTY `set -u` | Rebuild/update current script; non-TTY runs are supported |
| Package `build.sh` can’t find binary | `CARGO_TARGET_DIR` vs `rust/target` | Scripts honor `CARGO_TARGET_DIR`; rebuild helper/image docs |
| Taps fall through to home | No `EVIOCGRAB` / old binary | Rebuild current tree; grab is in `App::run` |
| Lock screen can’t unlock | Manual launch + grab | Expected until booklet integration; see [device.md](device.md) |

## 11. Tests map

| Area | Command / location |
|------|--------------------|
| Signals, layout, structural, app, input | `cargo test -p signalkit` |
| C ABI smoke | `cargo test -p signalkit --features capi` |
| Demo host stub | `cargo run -p signalkit-demo` |

## Related docs

- [concepts.md](concepts.md) — reactive model
- [usage.md](usage.md) — writing UI
- [device.md](device.md) — FBInk, touch, `App::run`
- [c-abi.md](c-abi.md) — C consumers
- [Demo package README](../../../apps/com.bd452.signalkitdemo/README.md)
- [Library package README](../../../apps/com.bd452.signalkit/README.md)
- Repo root [README.md](../../../README.md) — KPM repository build
- Repo [`Dockerfile`](../../../Dockerfile) — `kinstaller-build` image
