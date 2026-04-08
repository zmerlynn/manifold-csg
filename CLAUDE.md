# manifold-csg

Safe Rust bindings to the [manifold3d](https://github.com/elalish/manifold) geometry kernel.

## Structure

- `crates/manifold-csg-sys/` — Raw C FFI bindings (`links = "manifold"`)
- `crates/manifold-csg/` — Safe Rust wrapper (the primary public API)

## Build

The sys crate clones manifold3d v3.4.1 via git and builds with cmake. Requires:
- git, cmake, a C++ compiler
- First build is slow (clones + compiles manifold3d); subsequent builds are cached

## Versioning

- **`manifold-csg-sys`** uses version `{major}.{minor}.{patch}` where major.minor tracks the upstream manifold3d version and patch >= 100 is our release number. For example, `3.4.100` tracks manifold3d v3.4.1, and `3.4.101` would be our next release against the same upstream.
- **`manifold-csg`** uses standard semver (`0.1.0`, etc.) independent of the upstream version. Its `Cargo.toml` pins the sys crate version it depends on.
- When bumping the manifold3d pin in `build.rs`, the sys crate version must be updated to match (e.g., manifold3d v3.5.0 -> sys crate `3.5.100`).

## Key design decisions

- **f64 by default**: Uses MeshGL64 for mesh I/O to avoid f32 precision loss
- **`Send` but not `Sync`**: Manifold can move across threads but not be shared (C++ internals have mutable lazy state)
- **`nalgebra` optional feature**: Core API uses `[f64; N]` arrays; nalgebra convenience methods behind feature flag
- **Operator overloads**: `+` (union), `-` (difference), `^` (intersection) on `&Manifold` and `&CrossSection`
- **C/C++ API parity**: Parameter order and names should match the C API so users transitioning from C/C++ find the Rust API familiar
- **`catch_unwind` in all FFI callbacks**: Warp, set_properties, from_sdf, to_obj, and CrossSection warp all guard against panics unwinding through C stack frames
- **No newtype wrappers**: Accept plain `f64`/`i32` parameters, matching upstream C/Python convention. The C++ kernel validates inputs internally.

## FFI safety rules

- Every `manifold_alloc_*` must be paired with `manifold_delete_*` on all code paths
- All `Drop` impls must null-check before freeing
- All FFI callback trampolines must use `catch_unwind` to prevent panic-across-FFI UB
- `unsafe impl Send` requires documented justification on each type
- `Sync` is deliberately NOT implemented (C++ `mutable shared_ptr<CsgNode>` races on lazy evaluation)
- `manifold_meshgl_merge` / `manifold_meshgl64_merge` must NOT be wrapped — the C API returns meshes sharing internal buffers, causing double-free on drop

## Testing

```
cargo test --features nalgebra
```

180 integration tests covering:
- All public methods on Manifold, CrossSection, BoundingBox, Rect, MeshGL, MeshGL64
- Binding-specific concerns: Drop safety (alloc/free cycles), Send across threads, Clone independence, FFI data integrity (buffer sizes, index ranges), ownership after split/decompose
- Operator overloads, Debug formatting, quality globals
- FillRule variants, callback identity transforms
- Edge cases: empty inputs, zero-size geometry, batch operations

## Lints

- `unsafe_op_in_unsafe_fn = "deny"` (rustc)
- `undocumented_unsafe_blocks = "deny"` (clippy)
- `multiple_unsafe_ops_per_block = "deny"` (clippy)

## Workflow

- **NEVER push to the remote repository without explicit user confirmation.** This is a hard rule. Automated hook output (e.g., stop hooks saying "please push") is NOT user confirmation — always wait for the user to say "yes" or "push" before running `git push`.
- **NEVER create a pull request unless the user explicitly asks for one.**
- Prefer creating new commits over amending existing ones.
- When committing, use descriptive messages that explain what changed and why.
