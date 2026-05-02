# Plan: `wasm32-unknown-emscripten` target support

Status: **implemented**. The scaffold (panic + CI lane + this doc) landed
first; the implementation followed on the same branch. The original plan
text below is preserved as a historical record. The actual outcome is
summarized in [What shipped vs. predicted](#what-shipped-vs-predicted) at
the bottom.

## Why

Browser deployment of `manifold-csg`-using Rust apps. `wasm32-unknown-unknown`
is intractable for a non-trivial C++ kernel (no libc, no libcxx — see #30
for the upstream evidence and the in-thread experiment that came within ~31
undefined symbols of working). Emscripten is the next-best target: upstream
`manifold3d` already builds for it (their `manifoldcad.org` demo), so the
C++ side is solved. The work is teaching our `build.rs` to use the
Emscripten toolchain when the target asks for it.

Who this unblocks today:

- Rust apps that want a wasm flavor and don't depend on `wasm-bindgen` —
  CAD tools, geometry-only demos, headless processors, build-time
  preprocessors, server-side wasm runtimes that handle Emscripten's JS
  shims.

Who this is prerequisite for, but doesn't unblock yet:

- `wasm-bindgen`-based Rust web apps (Bevy, Leptos, Yew, etc.). These
  target `wasm32-unknown-unknown` because that's where wasm-bindgen
  works; until wasm-bindgen lands `wasm32-unknown-emscripten` support
  (in-flight upstream effort, not shipped), they can't directly link
  this target. They can use the JS-interop workaround (load manifold's
  Emscripten bundle as a separate `<script>`, call from Rust via
  wasm-bindgen JS bridge), but in that mode they'd use upstream's
  bundle from `manifoldcad.org`, not ours. When the wasm-bindgen
  effort lands, this PR is what makes the path immediately usable.

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
- `wasm-bindgen` interop glue — out of scope for the kernel crate.
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

### Local development

Brew has a current emscripten formula, which is the easiest path:

```bash
brew install emscripten            # 5.0.7 currently (matches CI pin)
rustup target add wasm32-unknown-emscripten
cargo build --target wasm32-unknown-emscripten -p manifold-csg-sys --no-default-features
```

Three lines, no shell-sourcing, `emcc`/`emcmake`/`emmake` go straight on
PATH and stay there. Iteration loop is ~30 s for a clean build, <5 s for
incremental build.rs changes.

If you need a specific emsdk version (e.g., to match CI exactly during a
debugging session), the raw emsdk install path still works:

```bash
git clone https://github.com/emscripten-core/emsdk.git
cd emsdk && ./emsdk install <version> && ./emsdk activate <version>
source ./emsdk_env.sh    # per-shell
```

CI uses the raw-emsdk path via `mymindstorm/setup-emsdk` because there's
no brew on GitHub runners. The exact version is pinned in `ci.yml` — if
you bump CI you should also test locally with the matching version.

### CI

A new `Emscripten (wasm32, scaffold)` lane is **already landed** with
`continue-on-error: true`. It uses `mymindstorm/setup-emsdk@v16` pinned
to the current stable emsdk version (`5.0.7`) since LLVM ABI shifts
between releases and floating `latest` breaks builds non-deterministically.

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
   `manifold_status` error codes. emcc requires exceptions to be enabled
   at *both* compile and link time to emit working exception code.
   Without it, thrown exceptions become trap-and-abort. Our build doesn't
   pass exception flags to the C++ compile by default (upstream only adds
   them under `MANIFOLD_DEBUG`). Will silently fail on any input that
   triggers a thrown exception.

   Two modes to choose from:

   - **`-fwasm-exceptions`** (preferred): native wasm exception handling
     via the `exception-handling` proposal. Broadly supported in browsers
     since ~2023 and is becoming the emcc default. More efficient, no JS
     trampoline. Try this first.
   - **`-fexceptions` + `-sDISABLE_EXCEPTION_CATCHING=0`** (fallback):
     older JS-based exception mode. Use only if the toolchain doesn't
     cooperate with `-fwasm-exceptions` (e.g., pinned emsdk too old).

   Likely fix: pass the chosen flag via `CXXFLAGS` for the emscripten
   build, plus the matching `cargo:rustc-link-arg-*` so it reaches the
   final link.

   **Avoid** setting `MANIFOLD_DEBUG=ON` to indirectly enable exceptions
   — it also turns on verbose `<iostream>` dumps, debug assertions, and
   the throw/catch paths in `csg_tree.cpp` / `polygon.cpp`, which inflate
   code size and pull in iostream runtime that release builds don't need.

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

## What shipped vs. predicted

The implementation landed in roughly an evening, well under the 1.5-day
estimate. Surprises and corrections worth recording:

**Estimate was too high.** Almost every step was easier than predicted.
Local development took the form of one `brew install emscripten`, one
`rustup target add`, one `cargo build`, iterate. Total iteration count
through "first failure → green tests" was around five.

**The C++ exception risk evaporated.** The plan flagged compile/link
mismatch on `-fwasm-exceptions` as the highest-risk silent correctness
bug. In practice, passing `-fwasm-exceptions` via `-DCMAKE_CXX_FLAGS`
on the C++ compile and forwarding it via `cargo:rustc-link-arg` on the
final link Just Worked. No exception-related failures. Both `-fexceptions`
JS-mode and `-fwasm-exceptions` were available in current emsdk; we picked
the newer wasm-native mode and it linked cleanly.

**Link-arg propagation needed the `DEP_<NAME>_<KEY>` pattern.** This was
flagged as a footgun but the actual solution is more principled than the
plan suggested. The shipped pattern:

- `manifold-csg-sys/build.rs` (which has `links = "manifold"`) emits
  `cargo:link_args=<space-separated flags>` when targeting emscripten.
  Cargo translates this into the `DEP_MANIFOLD_LINK_ARGS` env var visible
  to dependents.
- `manifold-csg/build.rs` (new file) reads `DEP_MANIFOLD_LINK_ARGS` and
  re-emits each token as `cargo:rustc-link-arg=...`. Those flags then
  reach any `cargo build` of a binary, test, cdylib, or example that
  depends on `manifold-csg`.
- End-user crates that produce their own bin/cdylib still need to forward
  the same way (re-read `DEP_MANIFOLD_LINK_ARGS` from their own build.rs,
  or set the flags in `.cargo/config.toml`). Documented in the README.

**Plain `cargo:rustc-link-arg` from a sys crate doesn't propagate.** This
was the first attempt and it silently does nothing for downstream consumers
— the flags only apply to the current crate's link invocations, and a
sys crate produces only an rlib (no link). The `DEP_<NAME>_<KEY>` indirect
pattern is the only stable way to push link flags through.

**Stack and initial memory were not in the plan but bit immediately.**
emcc's default 5 MiB stack overflows during one of our integration tests
(deep CSG recursion). Fixed with `-sSTACK_SIZE=33554432`. Then emcc
rejected that because INITIAL_MEMORY (default 16 MiB) must exceed
STACK_SIZE; bumped INITIAL_MEMORY to 64 MiB. Mirrors upstream's emscripten
cmake configuration. Should have been in the plan.

**Test harness "just worked" under Node.** The plan suggested the
Node-runner setup might be involved. In practice it's a single env var
(`CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_RUNNER=node`) and cargo's existing
`--target` infra handles the rest. 198 of 209 integration tests pass; the
11 that don't all use `std::thread::spawn`.

**Tests using `std::thread::spawn` were tagged `#[ignore]` for the target.**
The original plan note said "wasm32 default has no pthread support" — that
was lazy phrasing. Emscripten *can* support pthreads (with `-pthread`),
but enabling it requires consumers to serve their HTML with COOP/COEP
headers so the browser allows `SharedArrayBuffer`. Too much friction for
a default build, so we stay no-pthread; threading would be opt-in via a
future feature flag. The 11 ignored tests carry that explanation in their
ignore message.

**One CMake LTO knob mentioned ("`-DCMAKE_INTERPROCEDURAL_OPTIMIZATION=OFF`")
turned out unnecessary.** Default emcc + cmake plays nicely. Kept the
speculative note in the plan in case future toolchain bumps regress this.

**Drive-by `cfg!(target_os = …)` fix landed as planned.** The C++ stdlib
link block now reads `CARGO_CFG_TARGET_OS` / `CARGO_CFG_TARGET_ENV` from
env, which is what's correct under cross-compile. Was coincidentally
correct before because we never cross-compiled.

**Threading support remains explicitly out of scope.** A future PR can
add a `parallel-emscripten` (or similar) feature that opts into `-pthread`
+ `-sPTHREAD_POOL_SIZE` + the SharedArrayBuffer story. Not urgent; users
who want it can add the flags themselves via `.cargo/config.toml`.

## Production-readiness checklist

What "shipped + tested under Node" does *not* prove, in rough priority
order. Items are fine to address in follow-up PRs; nothing here blocks
the initial merge.

**Validated in this PR:**

- [x] Build for `wasm32-unknown-emscripten` (with and without `nalgebra`)
- [x] All non-thread integration tests pass under Node (198/209)
- [x] Existing examples (e.g. `basics`) build and produce correct
      numerical output under wasm (5-sphere union volume, mesh round-trip
      volume, convex hull volume all match host)
- [x] C++ exception path actually runs (out-of-bounds triangle index
      returns an error code rather than trapping the wasm module —
      `wasm_smoke_throw_path_returns_error_not_trap`)
- [x] Memory growth path actually runs (30-sphere batch_union completes,
      forcing the wasm linear memory past its 64 MiB initial size —
      `wasm_smoke_memory_growth_path`)

**Not yet validated:**

- [ ] **Real browser execution.** All testing has been under Node (V8).
      Chrome/Firefox/Safari use different wasm engines; differences can
      surface (often around exception handling and SIMD). A headless
      smoke test via `wasm-bindgen-test --chromedriver` (or Playwright)
      would close this gap. ~2 hours of CI work.
- [ ] **Numerical determinism vs. host.** Our integration tests use
      `assert_relative_eq!` with epsilons, so they pass even if wasm
      and host produce subtly different floats (e.g. from FP-determinism
      issues that elalish/manifold#1681 just fixed). Pick 5-10 tests,
      compare wasm output to host bit-identically, document expected
      drift if any. ~1 hour.
- [ ] **Long-running stability / leak behavior.** A single wasm instance
      doing thousands of CSG ops — does memory plateau, or grow
      unbounded? ASan testing on host already covers this for native;
      wasm has no equivalent. Practical test: run a soak harness in
      Node and watch the linear memory over time. ~2 hours.
- [ ] **Performance baseline.** Unknown ratio of wasm vs. native. Could
      be 1.5×, could be 10× slower. Add a `cargo bench` flavor for wasm
      and document the ratio in the README so users have realistic
      expectations. ~1-2 hours.
- [ ] **Multi-instance loading.** Untested whether two `manifold-csg`
      wasm modules can be loaded into the same JS page without one
      stomping the other's globals (manifold has process-globals like
      `meshIDCounter_`). Consumers building plugin systems would care.
- [x] **Real consumer integration.** A non-trivial Rust application
      using `manifold-csg` was successfully built against this branch
      and runs in wasm — exercises significantly more of the safe API
      than the integration tests do. `wasm-bindgen` consumers (Bevy,
      Leptos, Yew, etc.) remain blocked on upstream `wasm-bindgen`
      adding `wasm32-unknown-emscripten` support; non-`wasm-bindgen`
      consumers can use this target today.
- [ ] **Threading variant.** Out of scope here (deliberate). When/if
      someone wants `MANIFOLD_PAR=ON` in the browser, we'd need a new
      cargo feature that opts into `-pthread`,  `-sPTHREAD_POOL_SIZE`,
      and the SharedArrayBuffer / COOP+COEP story. Document in CLAUDE.md
      that this requires consumer cooperation at the HTTP layer.

## Release notes

When `manifold-csg` ships next with this change, the release notes should
mention:

- New target supported: `wasm32-unknown-emscripten` (browser via Emscripten).
  Build-only on `wasm32-unknown-unknown` is still not supported (see #30).
- `manifold-csg` now ships a `build.rs` (re-emits `DEP_MANIFOLD_LINK_ARGS`
  for downstream link steps). This adds a near-zero per-build overhead
  (one env-var lookup) on every consumer, regardless of target.
- New workspace `.cargo/config.toml` sets a `node` runner for the
  emscripten target. Workspace dependents inherit this; standalone
  consumers may override or ignore.
