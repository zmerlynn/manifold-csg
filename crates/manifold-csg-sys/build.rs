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

/// Pinned upstream version — can be a tag (e.g., "v3.4.1"), branch, or commit SHA.
const MANIFOLD_VERSION: &str = "65943caaab531ff9e135fe061868fde91760a372";

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
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let is_emscripten = target_os == "emscripten";

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
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=patches");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifold_src = out_dir.join("manifold-src");
    let build_dir = out_dir.join("build");

    // Invalidate cached source when the pinned commit changes.
    let commit_stamp = out_dir.join(".version-stamp");
    let old_commit = std::fs::read_to_string(&commit_stamp).unwrap_or_default();
    if old_commit.trim() != MANIFOLD_VERSION && manifold_src.exists() {
        if std::fs::remove_dir_all(&manifold_src).is_err() {
            let _ = Command::new("git")
                .args(["checkout", "."])
                .current_dir(&manifold_src)
                .status();
        }
        let _ = std::fs::remove_dir_all(&build_dir);
    }

    // Compute a hash of the patches directory so we can detect when patches
    // change and invalidate the cached (possibly stale) source checkout.
    let patches_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("patches");
    let patch_stamp = out_dir.join(".patch-stamp");
    let current_stamp: String = {
        let mut entries: Vec<String> = Vec::new();
        if let Ok(dir) = std::fs::read_dir(&patches_dir) {
            for e in dir.flatten() {
                let path = e.path();
                if path.extension().is_none_or(|ext| ext != "patch") {
                    continue;
                }
                if let Ok(contents) = std::fs::read_to_string(&path) {
                    entries.push(format!("{}:{}", path.display(), contents.len()));
                }
            }
        }
        entries.sort();
        entries.join(";")
    };
    let old_stamp = std::fs::read_to_string(&patch_stamp).unwrap_or_default();
    if current_stamp != old_stamp && manifold_src.exists() {
        // Patches changed — delete cached source so it gets re-cloned and re-patched.
        // If removal fails (e.g. read-only files on Windows), reset via git instead.
        if std::fs::remove_dir_all(&manifold_src).is_err() {
            let _ = Command::new("git")
                .args(["checkout", "."])
                .current_dir(&manifold_src)
                .status();
        }
        let _ = std::fs::remove_dir_all(&build_dir);
    }

    // Clone manifold3d if not already present.
    if !manifold_src.join("CMakeLists.txt").exists() {
        let status = Command::new("git")
            .args([
                "-c",
                "core.autocrlf=false",
                "clone",
                "https://github.com/elalish/manifold.git",
                manifold_src.to_str().unwrap(),
            ])
            .status()
            .expect("failed to run git clone for manifold3d");
        assert!(status.success(), "git clone manifold3d failed");

        // Checkout pinned commit.
        let status = Command::new("git")
            .args(["checkout", MANIFOLD_VERSION])
            .current_dir(&manifold_src)
            .status()
            .expect("failed to checkout manifold3d commit");
        assert!(status.success(), "git checkout manifold3d failed");

        let _ = std::fs::write(&commit_stamp, MANIFOLD_VERSION);
    }

    // Apply carry-patches (fixes awaiting upstream merge).
    let patches_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("patches");
    if patches_dir.exists() {
        let Ok(entries) = std::fs::read_dir(&patches_dir) else {
            panic!("failed to read patches directory");
        };
        let mut patches: Vec<_> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "patch"))
            .collect();
        patches.sort();
        for patch in &patches {
            let check = Command::new("git")
                .args([
                    "apply",
                    "--check",
                    "--ignore-whitespace",
                    "--whitespace=nowarn",
                ])
                .arg(patch)
                .current_dir(&manifold_src)
                .output()
                .expect("failed to check patch");
            if check.status.success() {
                let apply = Command::new("git")
                    .args(["apply", "--ignore-whitespace", "--whitespace=nowarn"])
                    .arg(patch)
                    .current_dir(&manifold_src)
                    .output()
                    .expect("failed to apply patch");
                assert!(
                    apply.status.success(),
                    "failed to apply patch {}: {}",
                    patch.display(),
                    String::from_utf8_lossy(&apply.stderr)
                );
                println!(
                    "cargo:warning=Applied patch: {}",
                    patch.file_name().unwrap_or_default().to_string_lossy()
                );
            } else {
                // Log why --check failed so we can diagnose CI issues.
                println!(
                    "cargo:warning=Patch skipped (already applied?): {} ({})",
                    patch.file_name().unwrap_or_default().to_string_lossy(),
                    String::from_utf8_lossy(&check.stderr).trim()
                );
            }
        }
    }
    // Record current patch state so we can detect changes on next build.
    let _ = std::fs::write(&patch_stamp, &current_stamp);

    // Configure with cmake.
    let mut parallel = env::var("CARGO_FEATURE_PARALLEL").is_ok();

    // Threading on emscripten requires SharedArrayBuffer + COOP/COEP HTTP
    // headers from the hosting page, which is too much friction to require
    // by default. Force MANIFOLD_PAR=OFF and warn if the user explicitly
    // asked for it.
    if is_emscripten && parallel {
        println!(
            "cargo:warning=manifold-csg-sys: 'parallel' feature is not supported on \
             {target}; building without TBB. Disable default-features or the \
             'parallel' feature to silence this warning."
        );
        parallel = false;
    }

    let mut cmake_args = vec![
        "-S".to_string(),
        manifold_src.to_str().unwrap().to_string(),
        "-B".to_string(),
        build_dir.to_str().unwrap().to_string(),
        "-DCMAKE_BUILD_TYPE=Release".to_string(),
        "-DMANIFOLD_TEST=OFF".to_string(),
        "-DMANIFOLD_PYBIND=OFF".to_string(),
        "-DMANIFOLD_JSBIND=OFF".to_string(),
        "-DMANIFOLD_CBIND=ON".to_string(),
        "-DMANIFOLD_CROSS_SECTION=ON".to_string(),
        "-DMANIFOLD_USE_BUILTIN_CLIPPER2=ON".to_string(),
        "-DBUILD_SHARED_LIBS=OFF".to_string(),
        "-DCMAKE_POSITION_INDEPENDENT_CODE=ON".to_string(),
    ];

    if parallel {
        cmake_args.push("-DMANIFOLD_PAR=ON".to_string());
        cmake_args.push("-DMANIFOLD_USE_BUILTIN_TBB=ON".to_string());
    } else {
        cmake_args.push("-DMANIFOLD_PAR=OFF".to_string());
    }

    if is_emscripten {
        // Compile manifold's C++ with native wasm exception handling. Manifold's
        // C wrapper translates internal C++ exceptions into status codes; without
        // this flag those throws would trap-and-abort the wasm module on invalid
        // input. Must match the link-time -fwasm-exceptions emitted below.
        cmake_args.push("-DCMAKE_CXX_FLAGS=-fwasm-exceptions".to_string());
    }

    // emcmake / emmake wrap cmake invocations to inject Emscripten's toolchain
    // file and substitute em++/emcc as the C++/C compiler.
    let make_cmake_cmd = |em_wrapper: &str| -> Command {
        if is_emscripten {
            let mut c = Command::new(em_wrapper);
            c.arg("cmake");
            c
        } else {
            Command::new("cmake")
        }
    };

    let cmake_output = make_cmake_cmd("emcmake")
        .args(&cmake_args)
        .output()
        .expect("failed to run cmake configure");
    if !cmake_output.status.success() {
        eprintln!("{}", String::from_utf8_lossy(&cmake_output.stderr));
        panic!("cmake configure failed");
    }

    // Build both manifold and manifoldc.
    let build_output = make_cmake_cmd("emmake")
        .args([
            "--build",
            build_dir.to_str().unwrap(),
            "--config",
            "Release",
            "--parallel",
        ])
        .output()
        .expect("failed to run cmake build");
    if !build_output.status.success() {
        eprintln!("{}", String::from_utf8_lossy(&build_output.stderr));
        panic!("cmake build failed");
    }

    // Find where libraries were placed and add search paths.
    //
    // On Unix, cmake puts libraries directly in the build subdirectories.
    // On MSVC, cmake's multi-config generator puts them in Release/ subdirs.
    // We use find_lib_recursive to handle both layouts reliably.
    let required_libs = ["manifoldc", "manifold", "Clipper2"];
    for lib_name in &required_libs {
        if let Some(lib_dir) = find_lib_recursive(&build_dir, lib_name) {
            println!("cargo:rustc-link-search=native={}", lib_dir.display());
        }
    }

    // Link order matters: manifoldc depends on manifold, which depends on Clipper2 and TBB.
    println!("cargo:rustc-link-lib=static=manifoldc");
    println!("cargo:rustc-link-lib=static=manifold");
    println!("cargo:rustc-link-lib=static=Clipper2");

    // TBB (builtin, for parallel CSG operations — only when "parallel" feature is enabled).
    // On different platforms, the TBB static library has different names:
    // Unix: libtbb.a, MSVC: tbb.lib or tbb12.lib or tbb12_static.lib
    if parallel {
        let tbb_names = ["tbb", "tbb12", "tbb12_static"];
        let mut found_tbb = false;
        for name in &tbb_names {
            if let Some(tbb_dir) = find_lib_recursive(&build_dir, name) {
                println!("cargo:rustc-link-search=native={}", tbb_dir.display());
                println!("cargo:rustc-link-lib=static={name}");
                found_tbb = true;
                break;
            }
        }
        if !found_tbb {
            // Fall back to letting the linker find it by the default name.
            println!("cargo:rustc-link-lib=static=tbb");
        }
    }

    // C++ standard library. Read target via env, not cfg! — cfg! evaluates at
    // build-script-host compile time, which silently lies under cross-compile.
    //
    // - MSVC links the C++ runtime automatically — no explicit link needed.
    // - Emscripten's emcc auto-links libc++ during the final wasm link.
    if !is_emscripten && target_env != "msvc" {
        if target_os == "macos" {
            println!("cargo:rustc-link-lib=c++");
        } else {
            println!("cargo:rustc-link-lib=stdc++");
        }
    }

    // Emscripten link flags. These need to reach the final rustc → emcc link
    // step in any binary/test/cdylib that depends on us, not just cmake's own
    // link step (which is a no-op here since BUILD_SHARED_LIBS=OFF).
    //
    // Plain `cargo:rustc-link-arg=` from a sys crate's build script does NOT
    // propagate to downstream link invocations — only `rustc-link-lib` and
    // `rustc-link-search` do. The proper sys-crate idiom is to expose the
    // flags as `links` metadata (here as DEP_MANIFOLD_LINK_ARGS, since this
    // crate has `links = "manifold"`), and have the safe wrapper crate's
    // build.rs read DEP_MANIFOLD_LINK_ARGS and re-emit `rustc-link-arg=` from
    // there. End-user binaries then need a similar build.rs (or a
    // `.cargo/config.toml` entry) to forward through to their own link.
    //
    // Documented in docs/plans/wasm-emscripten.md.
    if is_emscripten {
        let link_args: &[&str] = &[
            // Native wasm exception handling — must match -fwasm-exceptions
            // passed to the C++ compile above.
            "-fwasm-exceptions",
            // Allow the wasm linear memory to grow at runtime; the default
            // 16 MiB heap will OOM on the first non-trivial mesh.
            "-sALLOW_MEMORY_GROWTH=1",
            // Cap memory at the wasm32 ceiling (4 GiB) rather than the smaller
            // default, so growth doesn't trap on large boolean operations.
            "-sMAXIMUM_MEMORY=4294967296",
            // Bump the stack from emcc's small default (~5 MB). Manifold's
            // recursive CSG / topology routines hit stack overflow under the
            // default. Mirrors upstream's emscripten cmake configuration
            // (which uses 30 MB; round to 32 MiB).
            "-sSTACK_SIZE=33554432",
            // emcc requires INITIAL_MEMORY > STACK_SIZE, and the default
            // (16 MiB) is smaller than our stack. Bump to 64 MiB to give
            // headroom for stack + initial heap.
            "-sINITIAL_MEMORY=67108864",
        ];
        // Space-separated; downstream parses on whitespace. No flag here may
        // contain a literal space — if you ever need that (e.g. paths with
        // spaces in them), change to a different separator like ';' and update
        // crates/manifold-csg/build.rs to match.
        println!("cargo:link_args={}", link_args.join(" "));
    }
}
