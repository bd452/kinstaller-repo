//! Builds vendored FBInk as a static library and generates FFI bindings from
//! its header.
//!
//! FBInk source lives in the fbink KPM package's submodule
//! (`apps/com.bd452.fbink/vendor/FBInk`). We run its Makefile's `pic` target
//! (the same `KINDLE=true` / `CROSS_TC` cross build the fbink package uses),
//! then link `libfbink.a`. Bindings are generated with bindgen pointed at the
//! target triple so the FBInkConfig/FBInkState layouts match the device.
//!
//! This runs only when the `signalkit` crate is built with `--features fbink`
//! (i.e. the on-device cross build); host `cargo test` never compiles it.

use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    // rust/fbink-sys -> repo root
    let repo_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("fbink-sys must live at <repo>/rust/fbink-sys")
        .to_path_buf();
    let fbink_dir = repo_root
        .join("apps")
        .join("com.bd452.fbink")
        .join("vendor")
        .join("FBInk");
    let header = fbink_dir.join("fbink.h");

    assert!(
        header.exists(),
        "FBInk source missing at {} — run: git submodule update --init --recursive",
        fbink_dir.display()
    );

    // --- Build FBInk as a static library ---
    // CROSS_TC (e.g. arm-kindlehf-linux-gnueabihf) is exported by the app
    // build.sh; the matching gcc must be on PATH.
    // MINIMAL keeps the fixed-cell text path (DRAW + BITMAP) that we use via
    // fbink_print / fbink_cls / fbink_refresh, while dropping the OpenType
    // (FreeType) and image (zlib) code — so the static lib links against only
    // libm, with no extra runtime deps to ship to the device.
    // Cargo exports DEBUG=true/false to build scripts. FBInk's Makefile keys
    // its output dir on `ifdef DEBUG` — i.e. the *presence* of the variable,
    // not its value — so leaving it set makes `make` build into Debug/ while we
    // link Release/. Unset it (and OPT_LEVEL, harmless but same class) so FBInk
    // uses its Release path.
    let make_env_scrub = |cmd: &mut Command| {
        cmd.env_remove("DEBUG").env_remove("OPT_LEVEL");
    };

    // Clean first: FBInk's Release/ output is shared across targets and build
    // configs, and a stale object (e.g. one built with a different float ABI)
    // would be silently relinked, causing hard-to-diagnose ABI mismatches.
    let mut clean = Command::new("make");
    clean.current_dir(&fbink_dir).arg("clean");
    make_env_scrub(&mut clean);
    let _ = clean.status();

    // Build FBInk as a *PIC* static library (`pic` == `staticlib SHARED=true`).
    // The signalkit crate is also built as a cdylib (libsignalkit.so, the C
    // ABI), and linking non-PIC ARM objects into a shared object fails
    // ("recompile with -fPIC"). PIC objects link cleanly into both the cdylib
    // and the demo executable.
    let mut make = Command::new("make");
    make.current_dir(&fbink_dir)
        .arg("pic")
        .arg("KINDLE=true")
        .arg("MINIMAL=1")
        .arg("DRAW=1")
        .arg("BITMAP=1");
    make_env_scrub(&mut make);
    if let Ok(cross_tc) = env::var("CROSS_TC") {
        make.arg(format!("CROSS_TC={cross_tc}"));
    }
    // Parallelize with the jobserver Cargo provides.
    let status = make.status().expect("failed to spawn `make` for FBInk");
    assert!(status.success(), "FBInk `make pic` failed");

    let archive = fbink_dir.join("Release").join("libfbink.a");
    assert!(
        archive.exists(),
        "FBInk build did not produce {} — check the make output above",
        archive.display()
    );

    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::copy(&archive, out.join("libfbink.a")).expect("failed to copy FBInk archive to OUT_DIR");

    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=fbink");
    // FBInk's fixed-cell font path needs libm; no FreeType is pulled in because
    // we never call the OpenType (fbink_print_ot) API.
    println!("cargo:rustc-link-lib=dylib=m");

    // --- Generate bindings ---
    let target = env::var("TARGET").unwrap();
    // Parse the header for the *target*: set --target so struct layouts match,
    // and point clang at the cross toolchain's sysroot so it finds the target's
    // glibc headers (bits/libc-header-start.h, etc.) instead of the build host's.
    let mut clang_args = vec![format!("--target={target}")];
    if let Ok(cross_tc) = env::var("CROSS_TC") {
        if let Ok(out) = Command::new(format!("{cross_tc}-gcc"))
            .arg("-print-sysroot")
            .output()
        {
            let sysroot = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !sysroot.is_empty() {
                clang_args.push(format!("--sysroot={sysroot}"));
            }
        }
    }
    let bindings = bindgen::Builder::default()
        .header(header.to_string_lossy())
        .clang_args(&clang_args)
        .allowlist_function("fbink_.*")
        .allowlist_type("FBInk.*")
        .allowlist_var("(FBFD_AUTO|LAST_MARKER|FONT|FG_|BG_|WFM_|HW_).*")
        .prepend_enum_name(false)
        .derive_default(true)
        .generate()
        .expect("failed to generate FBInk bindings");

    bindings
        .write_to_file(out.join("bindings.rs"))
        .expect("failed to write bindings");

    println!("cargo:rerun-if-changed={}", header.display());
    println!("cargo:rerun-if-env-changed=CROSS_TC");
}
