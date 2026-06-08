# Offline / bring-your-own manifold build

Tracks issue #49 (offline build support), closely related to #45 (build-time levers).

## Problem

`crates/manifold-csg-sys/build.rs` unconditionally `git clone`s manifold3d from GitHub at build time (`build/fetch.rs::clone_manifold3d`). Any builder that forbids arbitrary network access during the build fails at the clone. The motivating report is **Nix**: a derivation only gets network access if it is fixed-output (a deterministic hash of its result is declared up front); a build script making arbitrary network requests breaks that model. nixpkgs already ships a working `manifold` package, but there was no way to hand it to this crate.

## Why this is tractable

The sys crate has **no build-time header dependency** - the FFI surface is hand-written `extern` declarations in `src/lib.rs`, not bindgen. (This is also why docs.rs builds with `--network=none`.) So linking an externally-provided manifold needs only the libraries, not headers or a `.pc` file (manifold ships neither a pkg-config file nor, usefully here, anything we must parse - it exports a CMake package config).

## External manifold sources (for the link path)

| | upstream flake (`github:elalish/manifold`) | nixpkgs `manifold` |
|---|---|---|
| Version | self-referential (`src = self`): pin the input to `v3.5.1` to match our pin exactly | 3.5.1 (tracks the tag) |
| C bindings (`manifoldc`) | `MANIFOLD_CBIND=ON` | inherited ON (`cmake_dependent_option`, CROSS_SECTION on) |
| Libs | shared (`BUILD_SHARED_LIBS=ON`) | shared |
| Clipper2 / TBB | system | system (propagated clipper2, onetbb) |

Both are **shared** + **system Clipper2/TBB**, whereas the from-source build here is **static** + **builtin** Clipper2/TBB. A shared `libmanifold` records its `Clipper2`/`tbb`/`stdc++` deps as `NEEDED`, so the link path only links `manifoldc` + `manifold` and lets the dynamic linker resolve the rest.

## The escape hatch (implemented)

Env-var driven, consulted only on host targets (the wasm-uu and emscripten lanes have their own build paths and return before this is reached).

**`MANIFOLD_CSG_LIB_DIR`** - link a pre-built install. Skips clone **and** cmake. Fully offline, no C++ compile. `MANIFOLD_CSG_LIB_KIND` selects `dylib` (default) or `static`; the static case additionally links `Clipper2` (+ `tbb`, probing `tbb`/`tbb12`/`tbb12_static` like the from-source build, under `parallel`). For `dylib`, the directory must also be on the runtime search path (rpath / `LD_LIBRARY_PATH`) - `link-search=native` is link-time only; Nix `buildInputs` arranges rpath automatically. This is the recommended path for Nix.

## Deferred: `MANIFOLD_CSG_SOURCE_DIR` (build from a supplied source tree)

A "skip the clone but still run cmake" override was prototyped and cut before merge. The problem: `build/build.rs` hardcodes `-DMANIFOLD_USE_BUILTIN_CLIPPER2=ON` (and builtin TBB under `parallel`), and manifold's `cmake/manifoldDeps.cmake` turns those into `FetchContent_Declare(... GIT_REPOSITORY ...)` of Clipper2 / oneTBB **at configure time**. So a source-dir build still hits the network for the deps - it doesn't deliver the offline goal, and there's no env lever for the caller to opt into system deps.

manifold *does* support system deps: with `MANIFOLD_USE_BUILTIN_CLIPPER2=OFF` / `MANIFOLD_USE_BUILTIN_TBB=OFF`, `manifoldDeps.cmake` runs `find_package` + a pkg-config fallback. So a proper version of this hatch (`MANIFOLD_CSG_SYSTEM_DEPS=1`) would flip both builtin flags off. Two things make it more than a flag:

- **Silent fallback footgun:** if Clipper2 builtin is off and the system copy isn't found, manifold's cmake *forces builtin back on and FetchContents anyway* (network returns silently); TBB just goes missing and parallel breaks. The lever only helps a builder that actually provides system clipper2/tbb (Nix does).
- **Link-set rework:** with system deps, `Clipper2`/`tbb` are shared system libs, not archives under `build_dir`, so `build/build.rs`'s `find_lib_recursive(build_dir, ...)` + `static=` link lines must switch to system/dynamic linking for that case.

Tracked as future work; the `MANIFOLD_CSG_LIB_DIR` path already covers the motivating Nix case.

## Nix consumption sketch

```nix
# manifold from the upstream flake, pinned to our exact MANIFOLD_VERSION:
#   inputs.manifold.url = "github:elalish/manifold/v3.5.1";
# or nixpkgs' pkgs.manifold (also 3.5.1).
buildRustPackage {
  # ...
  MANIFOLD_CSG_LIB_DIR = "${manifold}/lib";   # dylib by default
  buildInputs = [ manifold ];                 # rpath / NEEDED pulls clipper2 + tbb
}
```

## Decisions / open items for review

- **Version alignment:** the pin was bumped v3.5.0 -> v3.5.1 alongside this, so nixpkgs' `manifold` (3.5.1) and the flake at `v3.5.1` match our pin exactly - no ABI/version drift for the link path. A committed `flake.lock` pins the nixpkgs revision that resolves manifold 3.5.1, so the match is reproducible rather than dependent on whatever `nixos-unstable` resolves at `nix develop` time; bumping `MANIFOLD_VERSION` should be paired with a `nix flake update` to a nixpkgs that ships the new version.
- **Link kind default is `dylib`.** Correct for the Nix/distro case; the static escape (`MANIFOLD_CSG_LIB_KIND=static`) covers a caller who built a static manifold but then owns the Clipper2/TBB link set matching their build flags.
- **Presence is validated; version is not.** build.rs checks the lib dir actually contains the `manifoldc`/`manifold` libraries for the chosen kind (the exact filenames the linker resolves for `-l{name}`), failing with a build.rs diagnostic rather than an opaque downstream link error. It does NOT validate they match `MANIFOLD_VERSION` - we emit a `cargo:warning` putting that on the caller. A future enhancement could probe a version symbol, but there's no stable C API for it.
- **v3.5.1 adds 6 `manifold_execution_context_*` C functions** (ctx-aware FromMeshGL / LevelSet / Smooth factories). These ARE bound in this change as `Manifold::*_with_context` constructors (the existing `from_sdf` / `from_meshgl` / `smooth_f64` constructors were refactored to share a body that dispatches on an optional context pointer). The f32 `manifold_execution_context_smooth` is FFI-declared but not safe-wrapped, mirroring the unwrapped f32 `manifold_smooth`.
- **wasm-uu risk:** the lane passes `MANIFOLD_GIT_TAG=v3.5.1` to wasm-cxx-shim v0.5.0 (tested at v3.5.0). Patch-release carry-patches are expected to still apply but are unverified until the wasm-uu CI lane runs.
- **CI coverage:** a repo-root `flake.nix` provides a devShell that links nixpkgs' prebuilt `manifold` (3.5.1, matching our pin) via `MANIFOLD_CSG_LIB_DIR`, and the `nix-offline` CI job runs `nix develop -c cargo test --features nalgebra` through it - the one lane that exercises the from-source bypass end-to-end. A buildable `packages.default` (`nix build`) is intentionally omitted: `rustPlatform.buildRustPackage` needs a committed `Cargo.lock`, which this workspace gitignores by convention. Commit a lock first to add a package output.
