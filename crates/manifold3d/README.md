# manifold3d

Safe Rust bindings to the [manifold3d](https://github.com/elalish/manifold)
geometry kernel for constructive solid geometry (CSG).

This crate is a **facade** that re-exports [`manifold-csg`](https://crates.io/crates/manifold-csg).
Both crates expose the same API — use whichever name you prefer.

## Migrating from 0.0.x

If you were using the original `manifold3d` 0.0.x (by @NickUfer, transferred
to this project), the API has changed significantly. See the
[migration guide](https://github.com/zmerlynn/manifold-csg/blob/main/MIGRATION_FROM_0.0.6.md),
which is structured for AI-assisted migration.

Notable changes:
- Newtype wrappers (`PositiveF64`, `Point3`, `Vec3`, `Matrix4x3`, ...) replaced by plain primitives (`f64`, `[f64; 3]`, `[f64; 12]`)
- `manifold3d-macros` crate removed — warp/set_properties callbacks are now closures
- Method names aligned with the manifold3d C API (`new_cuboid` → `cube`, `convex_hull` → `hull`, etc.)
- `nalgebra_interop` feature renamed to `nalgebra`
- No `export` feature (upstream removed `MeshIO` in v3.4.0; use a dedicated mesh I/O crate)

## Quick start

```rust
use manifold3d::{Manifold, CrossSection, JoinType};

let cube = Manifold::cube(20.0, 20.0, 20.0, true);
let hole = Manifold::cylinder(30.0, 5.0, 5.0, 32, false);
let result = &cube - &hole;
```

## See also

- [Full documentation](https://docs.rs/manifold3d)
- [`manifold-csg`](https://crates.io/crates/manifold-csg) — same crate, different name
- [API coverage](https://github.com/zmerlynn/manifold-csg/blob/main/API_COVERAGE.md)

## License

Apache-2.0 OR MIT
