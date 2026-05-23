#[path = "build/build.rs"]
mod build;
#[path = "build/fetch.rs"]
mod fetch;
#[path = "build/patch.rs"]
mod patch;
#[path = "build/wasm.rs"]
mod wasm;

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Recursively search for a static library under `dir`.
///
/// Searches for `lib{name}.a` (Unix) and `{name}.lib` (MSVC).
fn find_lib_recursive(dir: &Path, name: &str) -> Option<PathBuf> {
    let unix_target = format!("lib{name}.a");
    let msvc_target = format!("{name}.lib");
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_lib_recursive(&path, name) {
                return Some(found);
            }
        } else if path
            .file_name()
            .is_some_and(|f| f == unix_target.as_str() || f == msvc_target.as_str())
        {
            return path.parent().map(Path::to_path_buf);
        }
    }
    None
}

fn main() {
    // docs.rs builds with --network=none, so we can't clone manifold3d.
    // The FFI declarations are just extern signatures — skip the C build
    // entirely and let rustdoc generate docs from the Rust source alone.
    if env::var("DOCS_RS").is_ok() {
        return;
    }

    // Read target info from cargo (build-script-host cfg! is wrong for cross-compiling).
    let target = env::var("TARGET").unwrap_or_default();
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let is_emscripten = target_os == "emscripten";
    // wasm32-unknown-unknown — bare wasm without WASI or Emscripten. Browser
    // target for wasm-bindgen consumers. Has its own dedicated build path
    // that consumes wasm-cxx-shim for the C/C++ runtime; the rest of this
    // function (clone manifold, host cmake, etc.) is bypassed.
    let is_wasm_unknown_unknown =
        target_arch == "wasm32" && target_os == "unknown" && target_env.is_empty();

    if is_wasm_unknown_unknown {
        wasm::build_wasm_unknown_unknown();
        return;
    }

    if is_emscripten {
        // emcmake/emmake wrap cmake to inject the Emscripten toolchain. They
        // come from the Emscripten SDK (`brew install emscripten` or the raw
        // emsdk install path; either way the binaries need to be on PATH).
        if Command::new("emcmake").output().is_err() {
            panic!(
                "Building for {target} requires the Emscripten SDK on PATH \
                 (emcmake, emmake, emcc). Install via `brew install emscripten`, \
                 or run `source emsdk_env.sh` from a raw emsdk checkout. \
                 See docs/plans/wasm-emscripten.md."
            );
        }
    }

    // Prevent unnecessary build script re-execution.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=build");
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=patches");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let patches_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("patches");
    let manifold_src = out_dir.join("manifold-src");
    let build_dir = out_dir.join("build");
    fetch::fetch(&out_dir, &manifold_src, &build_dir, &patches_dir);
    build::build(
        is_emscripten,
        &manifold_src,
        &build_dir,
        &target,
        &target_env,
        &target_os,
    )
}
