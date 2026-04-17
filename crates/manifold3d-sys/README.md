# manifold3d-sys

Raw FFI bindings to the [manifold3d](https://github.com/elalish/manifold) C API.

This crate is a **facade** that re-exports [`manifold-csg-sys`](https://crates.io/crates/manifold-csg-sys).
Both crates expose the same FFI surface — use whichever name you prefer.

## Migrating from 0.0.x

If you were using the original `manifold3d-sys` 0.0.x (by @NickUfer, transferred
to this project), the API has changed significantly. See the
[migration guide](https://github.com/zmerlynn/manifold-csg/blob/main/MIGRATION_FROM_0.0.6.md).

Notable changes:
- No `export` feature (upstream removed `MeshIO` in v3.4.0)
- No `static` feature (always statically linked)
- Version scheme now tracks upstream manifold3d version in major.minor

## See also

- [`manifold3d`](https://crates.io/crates/manifold3d) — safe Rust wrapper (start here)
- [`manifold-csg`](https://crates.io/crates/manifold-csg) / [`manifold-csg-sys`](https://crates.io/crates/manifold-csg-sys) — same crates, different names

## License

Apache-2.0 OR MIT
