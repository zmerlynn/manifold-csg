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
        } else if path.file_name().is_some_and(|f| {
            f == unix_target.as_str() || f == msvc_target.as_str()
        }) {
            return path.parent().map(Path::to_path_buf);
        }
    }
    None
}

fn main() {
    // Prevent unnecessary build script re-execution.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/lib.rs");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifold_src = out_dir.join("manifold-src");
    let build_dir = out_dir.join("build");

    // Clone manifold3d if not already present.
    if !manifold_src.join("CMakeLists.txt").exists() {
        let status = Command::new("git")
            .args([
                "clone",
                "--depth=1",
                "--branch=v3.4.1",
                "https://github.com/elalish/manifold.git",
                manifold_src.to_str().unwrap(),
            ])
            .status()
            .expect("failed to run git clone for manifold3d");
        assert!(status.success(), "git clone manifold3d failed");
    }

    // Configure with cmake.
    let parallel = env::var("CARGO_FEATURE_PARALLEL").is_ok();

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
        "-DBUILD_SHARED_LIBS=OFF".to_string(),
        "-DCMAKE_POSITION_INDEPENDENT_CODE=ON".to_string(),
    ];

    if parallel {
        cmake_args.push("-DMANIFOLD_PAR=ON".to_string());
        cmake_args.push("-DMANIFOLD_USE_BUILTIN_TBB=ON".to_string());
    } else {
        cmake_args.push("-DMANIFOLD_PAR=OFF".to_string());
    }

    let cmake_output = Command::new("cmake")
        .args(&cmake_args)
        .output()
        .expect("failed to run cmake configure");
    if !cmake_output.status.success() {
        eprintln!("{}", String::from_utf8_lossy(&cmake_output.stderr));
        panic!("cmake configure failed");
    }

    // Build both manifold and manifoldc.
    let build_output = Command::new("cmake")
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

    // C++ standard library.
    // MSVC links the C++ runtime automatically — no explicit link needed.
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=c++");
    } else if cfg!(not(target_env = "msvc")) {
        println!("cargo:rustc-link-lib=stdc++");
    }
}
