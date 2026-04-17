# manifold3d-macros

> **DEPRECATED.** This crate's macros have been removed from the `manifold3d` API.

The [`manifold3d`](https://crates.io/crates/manifold3d) crate (0.1+) uses
closures for the `warp` and `set_properties` callbacks instead of proc
macros, so there is nothing left for this crate to provide.

Using either macro (`#[manifold_warp]` or `#[manifold_manage_vertex_properties]`)
from this crate will produce a compile error pointing at the migration guide.

## Migration

See the [migration guide](https://github.com/zmerlynn/manifold-csg/blob/main/MIGRATION_FROM_0.0.6.md#callbacks-warp-and-set_properties).

**Before:**

```rust
#[manifold_warp]
struct MyWarp;
impl WarpVertex for MyWarp {
    fn warp_vertex(&self, p: Point3) -> Point3 { ... }
}
let warped = manifold.warp(Pin::new(&MyWarp));
```

**After:**

```rust
let warped = manifold.warp(|x, y, z| [x + 1.0, y, z]);
```

## License

Apache-2.0 OR MIT
