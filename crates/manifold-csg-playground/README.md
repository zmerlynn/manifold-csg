# manifold-csg-playground

Interactive in-browser demo of `manifold-csg` booleans:

- Pick two primitives (cube / sphere / cylinder) for slots A and B.
- Move/rotate/scale either slot with three.js `TransformControls` gizmos.
- Pick the boolean (union / difference / intersection) — the result mesh
  recomputes live each frame.

Built on top of PR #34's `wasm32-unknown-unknown` target support: no
Emscripten, no `wasm-bindgen`, just a small C-style FFI shim in
`src/lib.rs` and a single `.wasm` file that the browser loads directly.

## Quick start

```sh
# 1. Build (requires rustup target add wasm32-unknown-unknown + LLVM 20+)
bash crates/manifold-csg-playground/build.sh

# 2. Serve
bash crates/manifold-csg-playground/serve.sh

# 3. Open http://localhost:8000 in a browser.
```

If LLVM 20+ is not on `PATH`, point at it via env var:

```sh
WASM_CXX_SHIM_LLVM_BIN_DIR=/usr/lib/llvm-20/bin bash build.sh
```

See [`../../docs/plans/wasm-unknown-unknown.md`](../../docs/plans/wasm-unknown-unknown.md) and the workspace [`README.md`](../../README.md)
"Browser without Emscripten" section for the full toolchain story.

## Tests

There's a Node-side test suite covering the wasm C ABI and the JS glue
that uploads the boolean result into a `BufferGeometry`. Run with:

```sh
bash crates/manifold-csg-playground/test.sh
```

`test.sh` rebuilds the wasm, runs `npm install` once into a local
gitignored `node_modules/` (just brings in `three` so the geometry tests
can use a real `BufferGeometry`), then `node --test tests/*.test.mjs`.

## Layout

- `src/lib.rs` — C ABI exports (`alloc`, `dealloc`, `set_primitive`,
  `set_transform`, `set_op`, `rebuild`, `positions_ptr`, `positions_len`,
  `indices_ptr`, `indices_len`).
- `web/index.html` — UI controls + import map for three.js (loaded from
  unpkg CDN, no bundler).
- `web/main.js` — three.js scene + gizmo wiring.
- `web/rebuild.js` — pure module with the wasm/three.js glue (matrix
  conversion, geometry rebuild). Extracted so it's testable under Node.
- `tests/` — Node tests (`wasm.test.mjs`, `rebuild.test.mjs`).
- `build.sh` / `serve.sh` / `test.sh` — convenience scripts.

## Not for crates.io

`publish = false` in `Cargo.toml` — this crate exists to demo the kernel,
not to be a library.
