# Plan: `wasm32-unknown-unknown` target support

Status: **implemented**. Bare-wasm browser target, depending on
[`wasm-cxx-shim`](https://github.com/zmerlynn/wasm-cxx-shim) for the
C/C++ runtime layer. Companion to the earlier `wasm32-unknown-emscripten`
support — same end-goal (browser deployment), different mechanism.

## Why

`wasm32-unknown-unknown` is the canonical wasm target for browser-deployed
Rust apps because that's what `wasm-bindgen` works with. The previous
emscripten target was useful but doesn't compose with the wasm-bindgen
ecosystem (Bevy, Leptos, Yew, etc.). Supporting `wasm32-unknown-unknown`
directly unblocks those consumers.

The C++ runtime gap (`wasm32-unknown-unknown` ships no libc, no libcxx,
no libcxxabi) is filled by `wasm-cxx-shim` (pinned to v0.2.0) — a small,
independently-maintained library providing exactly the C/C++ runtime
subset that manifold3d (and similar C++-via-Rust crates) need. See the
shim's `docs/context.md` for the design background.

## Scope

**In:** Build path for `wasm32-unknown-unknown` that produces a wasm
module with zero unexpected imports. Carry-patches against the pinned
manifold + Clipper2 to gate iostream-using paths under `MANIFOLD_NO_IOSTREAM`.
A consumer-side `libcxx-extras.cpp` providing the libc++ source-file
symbols the shim deliberately doesn't ship. CI lane.

**Out:**

- Threading. `wasm32-unknown-unknown` with `wasm-bindgen` is single-threaded
  by design (Web Workers + SharedArrayBuffer is the threading story, and
  it requires consumer-side COOP/COEP HTTP headers — same as the
  emscripten threading variant we deferred).
- Exception runtime. Compiled with `-fno-exceptions`; implicit STL throws
  (`bad_alloc`, etc.) abort. Same as the freestanding pattern in the
  shim's `libcxx/__cxa_throw` stub.
- File I/O / OBJ I/O. The carry-patch wraps manifold's iostream-using
  paths under `MANIFOLD_NO_IOSTREAM`. Consumers can pass meshes via the
  binary `MeshGL64` API but not via OBJ files on this target.
- General `wasm-bindgen` interop glue. Out of scope for the kernel crate.
- Updates to `wasm-cxx-shim` itself (its symbol surface). When a downstream
  consumer surfaces a missing symbol, file an issue against
  zmerlynn/wasm-cxx-shim — the shim's maintenance model grows by demand.

## Implementation outline

### `build.rs`

A separate `build_wasm_unknown_unknown()` function dispatched early when
`CARGO_CFG_TARGET_ARCH=wasm32 && CARGO_CFG_TARGET_OS=unknown && CARGO_CFG_TARGET_ENV=""`.
Steps:

1. Sanity-check `cmake` and `clang` on PATH.
2. Clone `wasm-cxx-shim` (pinned to `v0.2.0`) into `OUT_DIR` and build its
   three components (libc, libm, libcxx) via cmake using the shim's own
   wasm32 toolchain file.
3. Clone Clipper2 separately at the SHA manifold pins, apply our
   `0002-clipper2-strip-iostream.patch`. Manifold's cmake reuses this
   pre-cloned source via `FETCHCONTENT_SOURCE_DIR_CLIPPER2`.
4. Clone manifold into a separate tree (`manifold-src-wasm32-uu`) — the
   host/emscripten path uses its own clone with different patches and
   build flags. Apply our existing carry-patches (`#1687`, `#1688`) plus
   the wasm32-uu-specific `0001-manifold-ifdef-iostream.patch`.
5. cmake-configure + build manifold using the shim's toolchain file +
   wasm-specific flags (`-fno-exceptions`, `-fno-rtti`, `-nostdlib`,
   `-nostdinc++`, `-DMANIFOLD_NO_IOSTREAM=1`, `-DCLIPPER2_MAX_DECIMAL_PRECISION=8`,
   `MANIFOLD_PAR=OFF`).
6. Compile `wasm32-uu/libcxx-extras.cpp` to a `.o`, archive it as
   `libcxx_extras.a` via `llvm-ar` (so cargo can emit it as a normal
   `rustc-link-lib=static` and control link order).
7. Emit `cargo:rustc-link-search` + `cargo:rustc-link-lib` directives in
   the validated order: `cxx_extras` → `manifoldc` → `manifold` →
   `Clipper2` → `wasm-cxx-shim-libcxx` → `wasm-cxx-shim-libc` →
   `wasm-cxx-shim-libm`.

### LLVM discovery

Mirrors `wasm-cxx-shim`'s `cmake/toolchain-wasm32.cmake` ladder:

1. `WASM_CXX_SHIM_LLVM_BIN_DIR` env override.
2. Per-platform probe: `/opt/homebrew/opt/llvm@N/bin`, `/usr/lib/llvm-N/bin`,
   etc. (newest version first).
3. PATH lookup as last resort (Apple's stock clang lacks libc++ headers
   in the expected layout, so PATH-discovered clang is unlikely to work
   on macOS but is correct on most Linux setups).

`WASM_CXX_SHIM_LIBCXX_HEADERS` env separately overrides just the libc++
header path for unusual installs.

### Troubleshooting / bug reports

The toolchain stack on this target (LLVM ≥ 18 with libc++ headers, plus a
matching `__config_site`) is precise enough that the most common failure
mode — system libc++ leaking in instead of LLVM's — is hard to diagnose
from the build log alone. Two diagnostic surfaces help:

- **`crates/manifold-csg-sys/wasm32-uu/diagnose.sh`** — a standalone bash
  script that walks the same LLVM probe ladder `build.rs` uses, dumps
  Rust/cargo/cmake versions, env vars, `~/.cargo/config.toml`, and any
  cached build state under `target/`. Run from the repo root and attach
  the output to a bug report:

  ```sh
  bash crates/manifold-csg-sys/wasm32-uu/diagnose.sh > bugreport.txt 2>&1
  ```

- **`build.rs` failure hook** — when any wasm32-uu cmake/clang step
  fails, `build.rs` prints (after cmake's own output) the resolved
  `clang++` path, libc++ headers path, the candidates probed, key env
  vars, and tails of `CMakeError.log`/`CMakeOutput.log` from the failing
  build dir. It also points at `diagnose.sh` for everything outside the
  build's own scope.

### `wasm32-uu/` directory contents

Vendored from the wasm-cxx-shim reference implementation
(`test/manifold-link/`, `test/smoke/`):

- `include/__config_site` — libc++ build-knob overrides (no threads, no
  filesystem, no localization, etc.). Mandatory for the `_LIBCPP_HAS_*=0`
  knobs that gate the symbols we don't ship.
- `include/__assertion_handler` — libc++ assertion handler that traps
  rather than calling into iostream-based abort paths.
- `include/mutex` — stub `<mutex>` providing no-op `std::mutex` /
  `std::recursive_mutex` / `lock_guard` / `scoped_lock` / `unique_lock`
  for builds without pthreads. Manifold references these even though it
  doesn't actually need them at runtime in our serial config.
- `libcxx-extras.cpp` — the libc++ source-file symbols (shared_ptr
  internals, `std::nothrow`, `std::align`, `bad_weak_ptr` key functions,
  `__throw_bad_alloc`) that the shim's `libcxx` deliberately doesn't
  ship. Each consumer ships their own.
- `patches/0001-manifold-ifdef-iostream.patch` — wraps manifold's
  iostream-using OBJ I/O paths under `MANIFOLD_NO_IOSTREAM`. Generated
  against our pinned manifold SHA `65943ca`. Three blocks:
  `bindings/c/manifoldc.cpp` C-API OBJ functions, `src/impl.cpp`
  `FromChars` template, `src/impl.cpp` `WriteOBJWithEpsilon` /
  `ReadOBJWithEpsilon` / `ReadOBJ` / `WriteOBJ` / `operator<<`.
- `patches/0002-clipper2-strip-iostream.patch` — strips `<iostream>` from
  Clipper2 headers. Verbatim from wasm-cxx-shim, generated against
  Clipper2 SHA `46f6391...` which is what manifold pins.

### Cargo.toml

A new `unstable-wasm-uu` feature on both `manifold-csg-sys` and
`manifold-csg` (forwarded through the `manifold3d-sys` / `manifold3d`
facades). Required to build for this target — `build.rs` panics with an
instructive error if the target is `wasm32-unknown-unknown` and the
feature is off. Also emits a `cargo:warning=` on every wasm32-uu build
even with the feature on, as a build-time reminder of the constraints
(carry-patches, no exception runtime, OBJ I/O disabled). Future
graduation to "stable" support is a matter of dropping the warning and
keeping the feature as a no-op for compatibility.

The `parallel` feature continues to exist; it's silently downgraded for
`wasm32-unknown-unknown` (TBB / threads aren't available on this target),
same as for emscripten.

### Tests

The existing 11 thread-using tests are already gated with
`#[cfg_attr(target_os = "emscripten", ignore)]`. They need an additional
guard for `wasm32-unknown-unknown`. The two `wasm_smoke_*` tests added
for emscripten validation (`wasm_smoke_throw_path_returns_error_not_trap`
and `wasm_smoke_memory_growth_path`) work as-is on this target — the
points they validate (exception path returns an error, memory growth
works) apply equally.

### CI

A new `Wasm32 (unknown-unknown)` lane on Ubuntu installing LLVM 20+ via
apt (`clang-20 lld-20 libc++-20-dev libc++abi-20-dev`), building the
crate for the new target, running tests under Node via
`CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=node`.

## Production-readiness

The previous emscripten plan's production-readiness checklist applies
here too (browser execution vs Node, numerical determinism, long-running
stability, performance baseline, multi-instance loading). Items marked
`[x]` for emscripten generally apply to this target too once the smoke
tests pass.

The most important remaining gap for `wasm32-unknown-unknown` specifically
is **wasm-bindgen integration end-to-end**: this PR delivers the
freestanding wasm artifact, but consumers using `wasm-bindgen` in their
own crate need to verify their integration pulls in our static archives
correctly (the `DEP_MANIFOLD_LINK_ARGS` pattern from the emscripten work
also applies here, but for different reasons — emscripten needs runtime
flags, this target needs the C++ runtime archives propagated). Tracking
that as a separate stage when a real wasm-bindgen consumer surfaces
issues.

## Release notes

When `manifold-csg` ships next with this change, the release notes should
mention:

- New target supported: `wasm32-unknown-unknown` (bare-wasm browser
  target compatible with `wasm-bindgen`).
- Build dependency: `wasm-cxx-shim` v0.2.0 (cloned via build.rs into
  `OUT_DIR`; no Cargo dependency).
- New required tooling for this target: a wasm-capable LLVM 20+ install
  (`brew install llvm` on macOS, `apt install clang-20 lld-20 libc++-20-dev`
  on Debian). docs.rs continues to build only for the host target via
  the existing `DOCS_RS` guard.
- Disabled features on this target: TBB / threads (same as emscripten),
  OBJ I/O via streams (use binary `MeshGL64` API instead), exceptions
  (implicit STL throws abort).
