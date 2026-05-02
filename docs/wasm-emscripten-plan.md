# Plan: `wasm32-unknown-emscripten` target support

Status: **scaffold landed; implementation deferred**. This document captures
the design before any of the actual `emcmake`/`emmake` plumbing is written.

## Why

A real Bevy user (issue #30) wants browser deployment. `wasm32-unknown-unknown`
is intractable for a non-trivial C++ kernel (no libc, no libcxx — see #30 for
the upstream evidence). Emscripten is the next-best target: upstream
`manifold3d` already builds for it (their `manifoldcad.org` demo), so the
C++ side is solved. The work is teaching our `build.rs` to use the
Emscripten toolchain when the target asks for it.

Even before `wasm-bindgen` adds emscripten support (in-flight, separate),
direct emscripten consumers — CAD tools, geometry-only demos, server-side
wasm runtimes that handle JS shims — can use this target.

## Scope

**In:** `wasm32-unknown-emscripten` build path through `build.rs`,
`MANIFOLD_PAR=OFF` only (no SharedArrayBuffer ceremony), build-only CI
verification, README docs section, no FFI changes.

**Out:**

- `wasm32-unknown-unknown` — intractable; no libc/libcxx.
- `wasm32-wasip1` / `wasm32-wasip2` — separate toolchain considerations,
  follow-up issue if anyone asks.
- Emscripten + threading (`MANIFOLD_PAR=ON`) — requires SharedArrayBuffer +
  COOP/COEP HTTP headers + `-sPTHREAD_POOL_SIZE`. Opt-in feature for a
  later PR.
- `wasm-bindgen` interop / Bevy glue — out of scope for the kernel crate.
- Running `cargo test` under Node — follow-up after build succeeds.

## Implementation outline

### `build.rs`

Detect the target early:

```rust
let target = env::var("TARGET").unwrap_or_default();
let is_emscripten = target == "wasm32-unknown-emscripten";
```

Force `parallel = false` for emscripten regardless of feature flag, with a
`cargo:warning` if the user explicitly asked for it.

Swap `cmake` → `emcmake cmake` for configure and `cmake --build` → `emmake
cmake --build` for the build step. `emcmake` injects Emscripten's cmake
toolchain file; `emmake` wraps the build invocation so emcc/em++ are used.

Sanity check that `emcmake` is on PATH and panic with a helpful install
pointer if not — most users hitting this will not have run
`source ./emsdk_env.sh` and need to be told.

Don't emit `cargo:rustc-link-lib=c++/stdc++` for emscripten — emcc auto-links
libcxx during the final wasm link.

### Linker flags

Emscripten link options upstream sets via `add_link_options(...)` only apply
to cmake's own link step — but we build static libs (`BUILD_SHARED_LIBS=OFF`),
so cmake never invokes a final link. The actual final link is rustc → emcc.
We need to forward those flags via cargo:

```rust
if is_emscripten {
    for flag in [
        "-sALLOW_MEMORY_GROWTH=1",
        "-sMAXIMUM_MEMORY=4294967296",
        "-fexceptions",
        "-sDISABLE_EXCEPTION_CATCHING=0",
    ] {
        println!("cargo:rustc-link-arg-bins={flag}");
        println!("cargo:rustc-link-arg-tests={flag}");
        println!("cargo:rustc-link-arg-cdylib={flag}");
    }
}
```

**Footgun**: `cargo:rustc-link-arg-*` only affects the *current* crate's
binaries. If a downstream crate produces its own `cdylib`/`bin`, those flags
don't propagate. Document this in the README.

### Drive-by bug fix

The current C++ stdlib link block uses `cfg!(target_os = ...)` — that
evaluates at *build-script-host compile time*, not at target time.
Coincidentally correct today because we never cross-compile. Should be
reading `CARGO_CFG_TARGET_OS` from env. Easy fix; needed anyway for
emscripten to skip the stdlib link.

### `Cargo.toml`

No schema change required. The `parallel` feature continues to exist but is
silently downgraded for emscripten via the `build.rs` warning above. Cargo
doesn't support per-target feature defaults, so the `build.rs` override is
the correct mechanism.

### CI

A new `Emscripten (wasm32, scaffold)` lane is **already landed** with
`continue-on-error: true`. It uses `mymindstorm/setup-emsdk@v14` pinned to
a specific emsdk version (`3.1.74`) since LLVM ABI shifts between releases
and floating `latest` breaks builds non-deterministically.

Once the implementation lands, remove `continue-on-error` and the lane
becomes a required check.

### Testing strategy

**Build-only verification in the initial implementation PR.** Justification:

- The crate has zero target-specific Rust code. A successful build proves
  the FFI surface compiles and links. Algorithmic correctness is covered by
  the host test matrix.
- Running emscripten output requires Node (for the generated JS shim) or a
  wasm runtime that supports emscripten's syscalls. wasmtime/WASI alone
  doesn't — emscripten emits JS glue for libc syscalls.
- A `CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_RUNNER=node` setup works but
  adds CI complexity. Worth doing eventually, not for the first PR.

### docs.rs

No change. docs.rs builds for the host (linux x86_64), never wasm. The
existing `if env::var("DOCS_RS").is_ok() { return; }` guard already covers
this — the new target-detection path only fires for the actual
emscripten target.

## Open questions and risks

1. **Static-lib LTO mismatch.** emcc's static archives can embed LLVM
   bitcode; if rustc's emcc-as-linker is on a different LLVM version,
   inscrutable link errors result. Mitigation: pin emsdk version in CI;
   document the matched-version requirement in README. Possibly pass
   `-DCMAKE_INTERPROCEDURAL_OPTIMIZATION=OFF` for emscripten to avoid
   bitcode entirely.

2. **C++ exception strategy mismatch — highest-risk silent correctness
   bug.** Manifold's C wrapper translates internal C++ exceptions into
   `manifold_status` error codes. emcc requires `-fexceptions` at *both*
   compile and link time to emit working exception code. Without it,
   thrown exceptions become trap-and-abort. Our build doesn't pass
   `-fexceptions` to the C++ compile by default (upstream only adds it
   under `MANIFOLD_DEBUG`). Will silently fail on any input that triggers
   a thrown exception. Likely fix: pass `-fexceptions` via `CXXFLAGS` for
   the emscripten build.

3. **`find_lib_recursive` and emcc archive layout.** emcc-built static
   archives are still `lib*.a` so this should Just Work. Verify after
   first successful configure.

4. **`cargo:rustc-link-arg-*` propagation across crate boundaries.** As
   noted, link-args don't reach downstream binaries automatically.
   Acceptable footgun; document it.

5. **`cmake` build-dependency unused.** `[build-dependencies] cmake = "0.1"`
   is currently imported but unused — `build.rs` invokes `Command::new("cmake")`
   directly. Tangential; not an emscripten issue. Don't touch in this PR.

## Effort estimate

Phased, with confidence:

- **Phase 1 — `build.rs` target detection + emcmake/emmake invocation:**
  1–2 hours. High confidence. Mechanical.
- **Phase 2 — first successful local build:** 2–6 hours. Medium confidence.
  Surprises live here: cmake passing host CFLAGS that emcc rejects,
  link-arg propagation iteration, exception handling discovery.
- **Phase 3 — CI lane green:** 1–3 hours. Standard setup-emsdk dance.
- **Phase 4 — README docs section:** 1–2 hours.
- **Phase 5 (optional) — Node-runner cargo test:** 2–8 hours. Defer.

Initial shippable PR (phases 1–4): **~1.5 days**, with a real risk of
stretching to **3 days** if the C++ exception story or LTO ABI mismatch
bites. Beyond that → ship a partial result and file a tracking issue.

## What's already landed (this scaffold PR)

1. `build.rs` panics with a clear "not yet implemented; see plan" message
   when `TARGET=wasm32-unknown-emscripten`. Better than silently invoking
   host cmake/clang and producing an unlinkable artifact.
2. CI lane added with `continue-on-error: true` so failures don't gate
   other PRs while the implementation is iterating.
3. This planning document.

## What lands next (the real implementation PR)

Branch: `emscripten-target-support` (already created, currently empty).

1. Replace the panic with the actual `emcmake`/`emmake` invocation per
   the outline above.
2. Add the link-arg forwarding for `-sALLOW_MEMORY_GROWTH=1` etc.
3. Drive-by: fix the `cfg!(target_os = ...)` → `CARGO_CFG_TARGET_OS`
   reads in the C++ stdlib link block.
4. Add README "Browser / WebAssembly" section.
5. Remove `continue-on-error: true` from the CI lane.
6. Add patches under `crates/manifold-csg-sys/patches/` only if upstream's
   `if(EMSCRIPTEN)` block has rough edges with our pinned SHA.
