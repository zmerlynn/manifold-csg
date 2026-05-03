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
        build_wasm_unknown_unknown();
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

// ── wasm32-unknown-unknown build path ────────────────────────────────────
//
// Bare wasm — no WASI, no Emscripten. Targets the wasm-bindgen Rust web
// ecosystem (Bevy, Leptos, Yew, etc., once wasm-bindgen lands emscripten
// support; usable today by any wasm-bindgen-style consumer that doesn't
// need wasm-bindgen itself). Depends on `wasm-cxx-shim` for the C/C++
// runtime layer that wasm32-unknown-unknown is missing (no libc, no
// libcxx, no libcxxabi).
//
// The build path here is fundamentally different from the host/emscripten
// path: a different toolchain (clang via wasm-cxx-shim's toolchain file),
// different cmake flags (-fno-exceptions, -fno-rtti, -nostdlib, -nostdinc++,
// MANIFOLD_PAR=OFF, etc.), additional dependencies (Clipper2 cloned
// separately so we can patch it), additional carry-patches to manifold
// (iostream gating), and a libcxx-extras.cpp consumer-side file that
// provides the libc++ source-file symbols (shared_ptr internals, etc.)
// the shim deliberately doesn't ship.
//
// See docs/plans/wasm-unknown-unknown.md for the design and
// `crates/manifold-csg-sys/wasm32-uu/` for the vendored helper files.
//
// Reference implementation: wasm-cxx-shim's
// `test/manifold-link/CMakeLists.txt` (the same recipe, expressed as a
// build.rs instead of a cmake file).

const WASM_CXX_SHIM_GIT: &str = "https://github.com/zmerlynn/wasm-cxx-shim.git";
const WASM_CXX_SHIM_TAG: &str = "v0.2.0";

// Clipper2 SHA — must match what manifold pins (cmake/manifoldDeps.cmake).
// If manifold bumps its Clipper2 pin, this must move in lockstep, or the
// FETCHCONTENT_SOURCE_DIR_CLIPPER2 override + iostream patch may fail to
// apply against the version manifold actually expects.
const CLIPPER2_GIT: &str = "https://github.com/AngusJohnson/Clipper2.git";
const CLIPPER2_SHA: &str = "46f639177fe418f9689e8ddb74f08a870c71f5b4";

fn build_wasm_unknown_unknown() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=patches");
    println!("cargo:rerun-if-changed=wasm32-uu");

    // wasm32-unknown-unknown support is provisional: the build path carries
    // patches against upstream manifold and Clipper2, ships without an
    // exception runtime (implicit STL throws abort), disables OBJ I/O, and
    // depends on a precise LLVM toolchain. Require an explicit feature flag
    // so this is acknowledged at the consumer's Cargo.toml.
    if env::var("CARGO_FEATURE_UNSTABLE_WASM_UU").is_err() {
        panic!(
            "Building manifold-csg-sys for wasm32-unknown-unknown requires \
             the `unstable-wasm-uu` cargo feature. Add it to your dependency:\n\
             \n    \
             manifold-csg = {{ version = \"...\", features = [\"unstable-wasm-uu\"] }}\n\
             \n\
             See the README's \"Browser without Emscripten\" section for the \
             constraints (no exceptions, no OBJ I/O, requires LLVM 20+)."
        );
    }
    println!(
        "cargo:warning=manifold-csg-sys: wasm32-unknown-unknown support is \
         provisional. Carry-patches against upstream manifold and Clipper2; \
         no exception runtime; OBJ I/O disabled. See README for details."
    );
    // Env vars that influence toolchain selection. Without these,
    // Cargo treats them as untracked and may skip rerunning build.rs
    // when the user changes their LLVM install pointer.
    println!("cargo:rerun-if-env-changed=WASM_CXX_SHIM_LLVM_BIN_DIR");
    println!("cargo:rerun-if-env-changed=WASM_CXX_SHIM_LIBCXX_HEADERS");

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let wasm_dir = manifest_dir.join("wasm32-uu");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // ---- Toolchain sanity check ----------------------------------------
    //
    // Resolve LLVM up front (instead of probing for stock `clang`, which on
    // macOS is Apple's clang and lacks libc++ headers in the layout we
    // need). find_llvm() panics with a focused install hint if the lookup
    // fails, so the user sees the right error before any cmake work
    // starts.
    if Command::new("cmake").arg("--version").output().is_err() {
        panic!(
            "Building for wasm32-unknown-unknown requires cmake on PATH. \
             Install via `brew install cmake` or your distro's package manager."
        );
    }
    let (clangpp, libcxx_headers, llvm_candidates) = find_llvm();

    // Diagnostic context for cmake/clang failures. Populated before any
    // cmake invocation so bail_with_diagnostics can report what build.rs
    // actually resolved (vs. what cmake might have re-discovered).
    let ctx = BuildContext {
        clangpp: clangpp.clone(),
        libcxx_headers: libcxx_headers.clone(),
        llvm_candidates,
    };

    // Warn loudly if the user has the `parallel` feature on for this
    // target (it's a no-op — wasm32-unknown-unknown has no threads — but
    // a silent downgrade is worse than a noisy one). Mirrors the
    // emscripten path's behavior.
    if env::var("CARGO_FEATURE_PARALLEL").is_ok() {
        println!(
            "cargo:warning=manifold-csg-sys: 'parallel' feature is not supported on \
             wasm32-unknown-unknown; building without TBB. Disable default-features \
             or the 'parallel' feature to silence this warning."
        );
    }

    // ---- Stage 1: clone + build wasm-cxx-shim ---------------------------

    let shim_src = out_dir.join("wasm-cxx-shim-src");
    let shim_build = out_dir.join("wasm-cxx-shim-build");
    let shim_stamp = out_dir.join(".shim-version-stamp");
    let shim_old = std::fs::read_to_string(&shim_stamp).unwrap_or_default();
    if shim_old.trim() != WASM_CXX_SHIM_TAG && shim_src.exists() {
        let _ = std::fs::remove_dir_all(&shim_src);
        let _ = std::fs::remove_dir_all(&shim_build);
    }
    if !shim_src.join("CMakeLists.txt").exists() {
        // Partial clone from a previous failed run? Wipe and retry.
        if shim_src.exists() {
            let _ = std::fs::remove_dir_all(&shim_src);
        }
        let status = Command::new("git")
            .args([
                "clone",
                "--depth",
                "1",
                "--branch",
                WASM_CXX_SHIM_TAG,
                WASM_CXX_SHIM_GIT,
                shim_src.to_str().unwrap(),
            ])
            .status()
            .expect("failed to run git clone for wasm-cxx-shim");
        assert!(status.success(), "git clone wasm-cxx-shim failed");
        let _ = std::fs::write(&shim_stamp, WASM_CXX_SHIM_TAG);
    }

    let shim_toolchain = shim_src.join("cmake/toolchain-wasm32.cmake");
    assert!(
        shim_toolchain.exists(),
        "wasm-cxx-shim missing toolchain at {}",
        shim_toolchain.display()
    );

    if !shim_build.join("libc/libwasm-cxx-shim-libc.a").exists() {
        let status = Command::new("cmake")
            .args([
                "-S",
                shim_src.to_str().unwrap(),
                "-B",
                shim_build.to_str().unwrap(),
                &format!("-DCMAKE_TOOLCHAIN_FILE={}", shim_toolchain.display()),
                "-DCMAKE_BUILD_TYPE=Release",
            ])
            .status()
            .expect("failed to run cmake configure for wasm-cxx-shim");
        if !status.success() {
            bail_with_diagnostics(&ctx, "wasm-cxx-shim cmake configure", &shim_build);
        }

        let status = Command::new("cmake")
            .args([
                "--build",
                shim_build.to_str().unwrap(),
                "--config",
                "Release",
                "--parallel",
            ])
            .status()
            .expect("failed to run cmake build for wasm-cxx-shim");
        if !status.success() {
            bail_with_diagnostics(&ctx, "wasm-cxx-shim cmake build", &shim_build);
        }
    }

    // ---- Stage 2: clone + patch Clipper2 --------------------------------
    //
    // We pin Clipper2 ourselves (matching what manifold pins) and apply
    // the iostream-strip patch. We then point manifold's FetchContent at
    // our pre-cloned source via FETCHCONTENT_SOURCE_DIR_CLIPPER2, so
    // manifold's cmake reuses our patched copy instead of re-cloning.

    let clipper2_src = out_dir.join("clipper2-src");
    let clipper2_stamp = out_dir.join(".clipper2-version-stamp");
    let clipper2_old = std::fs::read_to_string(&clipper2_stamp).unwrap_or_default();
    if clipper2_old.trim() != CLIPPER2_SHA && clipper2_src.exists() {
        let _ = std::fs::remove_dir_all(&clipper2_src);
    }
    if !clipper2_src.join("CMakeLists.txt").exists() {
        if clipper2_src.exists() {
            let _ = std::fs::remove_dir_all(&clipper2_src);
        }
        // Clipper2 doesn't have shallow tags here — clone full history then
        // checkout the SHA. The repo is small.
        let status = Command::new("git")
            .args(["clone", CLIPPER2_GIT, clipper2_src.to_str().unwrap()])
            .status()
            .expect("failed to run git clone for Clipper2");
        assert!(status.success(), "git clone Clipper2 failed");

        let status = Command::new("git")
            .args(["checkout", CLIPPER2_SHA])
            .current_dir(&clipper2_src)
            .status()
            .expect("failed to checkout Clipper2 SHA");
        assert!(status.success(), "git checkout Clipper2 failed");

        // Apply iostream patch.
        let patch = wasm_dir.join("patches/0002-clipper2-strip-iostream.patch");
        let status = Command::new("git")
            .args(["apply", "--ignore-whitespace", "-p0"])
            .arg(&patch)
            .current_dir(&clipper2_src)
            .status()
            .expect("failed to apply Clipper2 iostream patch");
        assert!(
            status.success(),
            "Clipper2 iostream patch failed to apply at SHA {CLIPPER2_SHA}"
        );

        let _ = std::fs::write(&clipper2_stamp, CLIPPER2_SHA);
    }

    // ---- Stage 3: clone + patch manifold (separate tree from host build)
    //
    // The host/emscripten codepath uses out_dir.join("manifold-src") and
    // applies its own patches (#1687, #1688). We need the same source but
    // also our wasm32-uu iostream patch, AND a separate cmake build dir so
    // the two configurations don't fight.
    //
    // Cleanest: a separate clone path so artifacts don't collide.

    let manifold_src = out_dir.join("manifold-src-wasm32-uu");
    let manifold_build = out_dir.join("manifold-build-wasm32-uu");
    let manifold_stamp = out_dir.join(".manifold-wasm32-uu-version-stamp");
    let manifold_old = std::fs::read_to_string(&manifold_stamp).unwrap_or_default();
    if manifold_old.trim() != MANIFOLD_VERSION && manifold_src.exists() {
        let _ = std::fs::remove_dir_all(&manifold_src);
        let _ = std::fs::remove_dir_all(&manifold_build);
    }
    if !manifold_src.join("CMakeLists.txt").exists() {
        if manifold_src.exists() {
            let _ = std::fs::remove_dir_all(&manifold_src);
        }
        let status = Command::new("git")
            .args([
                "-c",
                "core.autocrlf=false",
                "clone",
                "https://github.com/elalish/manifold.git",
                manifold_src.to_str().unwrap(),
            ])
            .status()
            .expect("failed to run git clone for manifold (wasm32-uu)");
        assert!(status.success(), "git clone manifold failed");

        let status = Command::new("git")
            .args(["checkout", MANIFOLD_VERSION])
            .current_dir(&manifold_src)
            .status()
            .expect("failed to checkout manifold pinned commit");
        assert!(status.success(), "git checkout manifold failed");

        // Apply our existing carry-patches first (#1687, #1688), then the
        // wasm32-uu-specific ones.
        let host_patches_dir = manifest_dir.join("patches");
        if host_patches_dir.exists() {
            let mut patches: Vec<_> = std::fs::read_dir(&host_patches_dir)
                .unwrap()
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|e| e == "patch"))
                .collect();
            patches.sort();
            for patch in &patches {
                let status = Command::new("git")
                    .args(["apply", "--ignore-whitespace", "--whitespace=nowarn"])
                    .arg(patch)
                    .current_dir(&manifold_src)
                    .status()
                    .expect("failed to apply carry-patch");
                assert!(
                    status.success(),
                    "failed to apply carry-patch {}",
                    patch.display()
                );
            }
        }

        // wasm32-uu iostream patch.
        let patch = wasm_dir.join("patches/0001-manifold-ifdef-iostream.patch");
        let status = Command::new("git")
            .args(["apply", "--ignore-whitespace", "-p0"])
            .arg(&patch)
            .current_dir(&manifold_src)
            .status()
            .expect("failed to apply wasm32-uu iostream patch to manifold");
        assert!(
            status.success(),
            "wasm32-uu iostream patch failed to apply against manifold @ {MANIFOLD_VERSION}"
        );

        let _ = std::fs::write(&manifold_stamp, MANIFOLD_VERSION);
    }

    // ---- Stage 4: cmake-configure + build manifold for wasm32-uu --------
    //
    // The compile flags here mirror what wasm-cxx-shim's
    // test/manifold-link/CMakeLists.txt sets via add_compile_options. We
    // pass them via CMAKE_C_FLAGS_INIT / CMAKE_CXX_FLAGS_INIT so they
    // reach manifold's compile rules without needing a wrapper
    // CMakeLists.txt.

    // -nostdlibinc (clang-specific) drops standard system include
    // directories but preserves the compiler resource dir, so builtin
    // headers like <stdint.h> and <stddef.h> still resolve. Belt-and-
    // suspenders alongside -nostdinc++: even if the explicit -isystem
    // chain misbehaves, the host C and C++ system paths stay excluded.
    let cxxflags = format!(
        "-fno-exceptions -fno-rtti -nostdlib -nostdinc++ -nostdlibinc \
         -DMANIFOLD_NO_IOSTREAM=1 \
         -DCLIPPER2_MAX_DECIMAL_PRECISION=8 \
         -isystem {wasm_inc} \
         -isystem {libcxx_inc} \
         -isystem {shim_libm_inc} \
         -isystem {shim_libc_inc}",
        wasm_inc = wasm_dir.join("include").display(),
        libcxx_inc = libcxx_headers.display(),
        shim_libm_inc = shim_src.join("libm/include").display(),
        shim_libc_inc = shim_src.join("libc/include").display(),
    );
    let cflags = format!(
        "-nostdlib -nostdlibinc \
         -isystem {shim_libc_inc}",
        shim_libc_inc = shim_src.join("libc/include").display(),
    );

    let status = Command::new("cmake")
        .args([
            "-S",
            manifold_src.to_str().unwrap(),
            "-B",
            manifold_build.to_str().unwrap(),
            &format!("-DCMAKE_TOOLCHAIN_FILE={}", shim_toolchain.display()),
            "-DCMAKE_BUILD_TYPE=Release",
            &format!("-DCMAKE_C_FLAGS_INIT={cflags}"),
            &format!("-DCMAKE_CXX_FLAGS_INIT={cxxflags}"),
            // Use our pre-patched Clipper2 instead of letting manifold clone its own.
            &format!(
                "-DFETCHCONTENT_SOURCE_DIR_CLIPPER2={}",
                clipper2_src.display()
            ),
            "-DMANIFOLD_TEST=OFF",
            "-DMANIFOLD_PYBIND=OFF",
            "-DMANIFOLD_JSBIND=OFF",
            "-DMANIFOLD_CBIND=ON",
            "-DMANIFOLD_CROSS_SECTION=ON",
            "-DMANIFOLD_PAR=OFF",
            "-DMANIFOLD_USE_BUILTIN_CLIPPER2=ON",
            "-DBUILD_SHARED_LIBS=OFF",
            "-DCMAKE_POSITION_INDEPENDENT_CODE=OFF",
            // Clipper2 default-on options pull in things we don't have.
            "-DCLIPPER2_TESTS=OFF",
            "-DCLIPPER2_UTILS=OFF",
            "-DCLIPPER2_EXAMPLES=OFF",
        ])
        .status()
        .expect("failed to run cmake configure for manifold (wasm32-uu)");
    if !status.success() {
        bail_with_diagnostics(&ctx, "manifold cmake configure", &manifold_build);
    }

    let status = Command::new("cmake")
        .args([
            "--build",
            manifold_build.to_str().unwrap(),
            "--config",
            "Release",
            "--parallel",
        ])
        .status()
        .expect("failed to run cmake build for manifold (wasm32-uu)");
    if !status.success() {
        bail_with_diagnostics(&ctx, "manifold cmake build", &manifold_build);
    }

    // ---- Stage 5: compile + archive libcxx-extras.cpp -------------------
    //
    // libcxx-extras provides the libc++ source-file symbols (shared_ptr
    // internals, std::nothrow, etc.) the shim deliberately doesn't ship.
    // We compile it here and wrap the .o in a static archive so we can
    // emit it via cargo:rustc-link-lib=static and let cargo control
    // link order alongside the other archives.

    let extras_o = out_dir.join("libcxx-extras.o");
    let extras_a = out_dir.join("libcxx_extras.a");
    let extras_cpp = wasm_dir.join("libcxx-extras.cpp");

    let status = Command::new(&clangpp)
        .args([
            "--target=wasm32-unknown-unknown",
            "-std=c++17",
            "-Os",
            "-fno-exceptions",
            "-fno-rtti",
            "-nostdlib",
            "-nostdinc++",
            "-nostdlibinc",
        ])
        .arg(format!("-isystem{}", wasm_dir.join("include").display()))
        .arg(format!("-isystem{}", libcxx_headers.display()))
        .arg(format!(
            "-isystem{}",
            shim_src.join("libm/include").display()
        ))
        .arg(format!(
            "-isystem{}",
            shim_src.join("libc/include").display()
        ))
        .arg("-c")
        .arg(&extras_cpp)
        .arg("-o")
        .arg(&extras_o)
        .status()
        .expect("failed to compile libcxx-extras.cpp");
    if !status.success() {
        bail_with_diagnostics(&ctx, "libcxx-extras compile", &out_dir);
    }

    let _ = std::fs::remove_file(&extras_a);
    // Use the llvm-ar that ships with our clang (next to it in the LLVM
    // bin/ dir). System `ar` won't produce wasm-friendly archives.
    let llvm_ar = clangpp
        .parent()
        .map(|d| d.join("llvm-ar"))
        .filter(|p| p.exists())
        .unwrap_or_else(|| PathBuf::from("llvm-ar"));
    let status = Command::new(&llvm_ar)
        .args(["rcs"])
        .arg(&extras_a)
        .arg(&extras_o)
        .status()
        .unwrap_or_else(|e| panic!("failed to run {} for libcxx-extras: {e}", llvm_ar.display()));
    if !status.success() {
        bail_with_diagnostics(&ctx, "libcxx-extras llvm-ar", &out_dir);
    }

    // ---- Stage 6: emit cargo metadata -----------------------------------
    //
    // Link order matters for static archives. Wasm-ld processes archives
    // left-to-right and pulls .o files as needed by previously-seen
    // undefined symbols. The order below mirrors wasm-cxx-shim's
    // test/manifold-link/CMakeLists.txt:
    //
    //   user obj/rlib (Rust crate) → libcxx_extras → manifoldc → manifold
    //     → Clipper2 → wasm-cxx-shim-libcxx → wasm-cxx-shim-libc
    //     → wasm-cxx-shim-libm

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=cxx_extras");

    let manifold_lib_dirs = [
        manifold_build.join("bindings/c"), // libmanifoldc.a
        manifold_build.join("src"),        // libmanifold.a
        manifold_build.join("_deps/clipper2-build"),
    ];
    for d in &manifold_lib_dirs {
        if d.exists() {
            println!("cargo:rustc-link-search=native={}", d.display());
        }
    }
    println!("cargo:rustc-link-lib=static=manifoldc");
    println!("cargo:rustc-link-lib=static=manifold");
    println!("cargo:rustc-link-lib=static=Clipper2");

    let shim_lib_dirs = [
        shim_build.join("libcxx"),
        shim_build.join("libc"),
        shim_build.join("libm"),
    ];
    for d in &shim_lib_dirs {
        println!("cargo:rustc-link-search=native={}", d.display());
    }
    println!("cargo:rustc-link-lib=static=wasm-cxx-shim-libcxx");
    println!("cargo:rustc-link-lib=static=wasm-cxx-shim-libc");
    println!("cargo:rustc-link-lib=static=wasm-cxx-shim-libm");

    // No `cargo:rustc-link-lib=c++/stdc++` on this target — wasm-cxx-shim
    // covers the C++ runtime. (Same reason emscripten skips it: emcc's
    // libc++ is auto-linked; ours is provided by the shim.)
}

/// Diagnostic context populated up-front in `build_wasm_unknown_unknown()`,
/// passed to `bail_with_diagnostics()` so cmake/clang failures emit the
/// resolved toolchain paths the user actually needs to debug.
struct BuildContext {
    clangpp: PathBuf,
    libcxx_headers: PathBuf,
    /// Bin dirs the LLVM probe actually checked, in the order they were tried.
    /// Surfaces the discovery ladder without making the user re-derive it.
    llvm_candidates: Vec<PathBuf>,
}

/// Print a diagnostic dump and panic. Called from each cmake/clang
/// failure site in the wasm32-uu path.
///
/// We don't capture cmake's stdout/stderr (they stream live), so this
/// dump comes *after* whatever cmake/make printed. It adds context the
/// user can't see otherwise: what build.rs resolved as the LLVM
/// toolchain, the env vars that influence it, and tails of cmake's own
/// configure-time logs.
fn bail_with_diagnostics(ctx: &BuildContext, stage: &str, build_dir: &Path) -> ! {
    eprintln!("\n=== manifold-csg-sys: wasm32-unknown-unknown build failed ({stage}) ===");
    eprintln!("clang++:        {}", ctx.clangpp.display());
    eprintln!("libc++ headers: {}", ctx.libcxx_headers.display());
    eprintln!("build dir:      {}", build_dir.display());
    eprintln!("LLVM candidates probed (in order):");
    for c in &ctx.llvm_candidates {
        eprintln!("  {}", c.display());
    }
    eprintln!("environment:");
    for k in [
        "WASM_CXX_SHIM_LLVM_BIN_DIR",
        "WASM_CXX_SHIM_LIBCXX_HEADERS",
        "CC",
        "CXX",
        "CFLAGS",
        "CXXFLAGS",
        "LDFLAGS",
        "RUSTFLAGS",
    ] {
        let v = env::var(k).unwrap_or_else(|_| "<unset>".into());
        eprintln!("  {k}={v}");
    }
    // cmake's own diagnostic logs — most useful for configure-time
    // failures (try-compile, missing tools). For build-time compile
    // errors the relevant output already streamed live above; these
    // will simply be absent or stale.
    for log in ["CMakeFiles/CMakeError.log", "CMakeFiles/CMakeOutput.log"] {
        let p = build_dir.join(log);
        if let Ok(contents) = std::fs::read_to_string(&p) {
            let lines: Vec<&str> = contents.lines().collect();
            let start = lines.len().saturating_sub(200);
            eprintln!(
                "\n--- {} (last 200 lines) ---\n{}",
                p.display(),
                lines[start..].join("\n")
            );
        }
    }
    eprintln!(
        "\nFor a full bug report, run:\n  \
         bash crates/manifold-csg-sys/wasm32-uu/diagnose.sh > bugreport.txt 2>&1\n\
         and attach bugreport.txt to the issue.\n"
    );
    panic!("wasm32-unknown-unknown build failed at stage: {stage}");
}

/// Locate an LLVM install with wasm32 support, returning
/// (clang++ path, libc++ headers dir, candidates probed in order).
///
/// The candidates list is returned alongside the resolved paths so that
/// `bail_with_diagnostics()` can show what the probe ladder considered
/// even on a successful resolve (which still might be wrong — e.g. an
/// LLVM that's too old, or a libc++ that doesn't actually match).
///
/// Mirrors wasm-cxx-shim's toolchain-wasm32.cmake discovery ladder so we
/// pick the same LLVM the cmake build is using. Order:
///   1. `WASM_CXX_SHIM_LLVM_BIN_DIR` env var (explicit override)
///   2. Common per-platform locations:
///        - macOS: /opt/homebrew/opt/llvm[@N]/bin
///        - Linux Debian-family: /usr/lib/llvm-N/bin
///   3. PATH lookup (Apple's stock clang lacks libc++ headers, so this is
///      a last resort)
///
/// `WASM_CXX_SHIM_LIBCXX_HEADERS` env var separately overrides just the
/// header path, for cases where the user wants a non-default libc++.
fn find_llvm() -> (PathBuf, PathBuf, Vec<PathBuf>) {
    let candidates = candidate_llvm_bin_dirs();

    if let Ok(headers) = env::var("WASM_CXX_SHIM_LIBCXX_HEADERS") {
        let headers = PathBuf::from(headers);
        let clangpp = which("clang++")
            .or_else(|| which("clang"))
            .expect("clang++/clang not found on PATH");
        warn_if_system_libcxx(&headers);
        return (clangpp, headers, candidates);
    }

    for bin_dir in &candidates {
        let clangpp = bin_dir.join("clang++");
        if !clangpp.exists() {
            continue;
        }
        let llvm_root = bin_dir.parent().unwrap();
        // LLVM ships libc++ headers either at <root>/include/c++/v1 or
        // <root>/lib/c++/v1, depending on layout. Try both.
        for rel in ["include/c++/v1", "lib/c++/v1"] {
            let headers = llvm_root.join(rel);
            if headers.join("vector").exists() {
                warn_if_system_libcxx(&headers);
                return (clangpp, headers, candidates);
            }
        }
    }

    panic!(
        "Could not find an LLVM install with libc++ headers and wasm32 support.\n\
         Tried: {candidates:?}\n\
         Install via:\n\
         \x20  - macOS:  brew install llvm  (then add to PATH per brew's instructions)\n\
         \x20  - Debian: apt install clang-20 lld-20 libc++-20-dev libc++abi-20-dev\n\
         Or set WASM_CXX_SHIM_LLVM_BIN_DIR to the directory containing clang++\n\
         (and ensure ../include/c++/v1 contains libc++ headers).\n\
         See docs/plans/wasm-unknown-unknown.md."
    );
}

/// Warn if `headers` (after symlink resolution) lives under a system
/// include path. On Debian-family distros, `/usr/lib/llvm-N/include/c++/v1`
/// is sometimes a symlink to `/usr/include/c++/v1` — meaning even though
/// we pass a versioned LLVM path to `-isystem`, clang ends up reading the
/// system libc++. That's the failure mode we hit when the system libc++
/// is newer (or older) than what our vendored `__config_site` covers.
///
/// We warn rather than reject because the symlink layout sometimes works
/// fine; only flag it so the user can correlate the warning with a build
/// failure and override via `WASM_CXX_SHIM_LIBCXX_HEADERS`.
fn warn_if_system_libcxx(headers: &Path) {
    let canonical = std::fs::canonicalize(headers).unwrap_or_else(|_| headers.to_path_buf());
    if canonical.starts_with("/usr/include/") || canonical.starts_with("/usr/local/include/") {
        println!(
            "cargo:warning=manifold-csg-sys: libc++ headers at {} resolve to a system path \
             ({}). System libc++ may be incompatible with our vendored __config_site; \
             if the build fails with _LIBCPP_* config errors, set \
             WASM_CXX_SHIM_LIBCXX_HEADERS to a non-symlinked LLVM-versioned path.",
            headers.display(),
            canonical.display()
        );
    }
}

fn candidate_llvm_bin_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(d) = env::var("WASM_CXX_SHIM_LLVM_BIN_DIR") {
        out.push(PathBuf::from(d));
    }
    // Newest first; first match wins. Mirrors the toolchain file's range.
    for v in [22, 21, 20, 19, 18] {
        // macOS Homebrew (Apple Silicon and Intel)
        out.push(PathBuf::from(format!("/opt/homebrew/opt/llvm@{v}/bin")));
        out.push(PathBuf::from(format!("/usr/local/opt/llvm@{v}/bin")));
        // Debian-family
        out.push(PathBuf::from(format!("/usr/lib/llvm-{v}/bin")));
    }
    // Unversioned brew paths
    out.push(PathBuf::from("/opt/homebrew/opt/llvm/bin"));
    out.push(PathBuf::from("/usr/local/opt/llvm/bin"));
    // Last-resort PATH lookup. (Nested-if rather than let-chain so we
    // stay compatible with our 1.85 MSRV; let-chains landed in 1.88.)
    #[allow(clippy::collapsible_if)]
    if let Some(p) = which("clang++") {
        if let Some(parent) = p.parent() {
            out.push(parent.to_path_buf());
        }
    }
    out
}

fn which(name: &str) -> Option<PathBuf> {
    let output = Command::new(if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    })
    .arg(name)
    .output()
    .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8(output.stdout).ok()?;
    Some(PathBuf::from(path.lines().next()?.trim()))
}
