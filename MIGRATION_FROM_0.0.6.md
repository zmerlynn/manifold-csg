# Migration guide: `manifold3d` 0.0.6 → 0.1+

This document covers migrating from the original `manifold3d` 0.0.x line
(maintained by @NickUfer, now transferred to this project) to `manifold3d`
0.1+ (this crate).

**This guide is structured so an AI coding assistant can read it and migrate
code automatically.** If you have a Claude/Copilot/Cursor-style tool,
point it at this file and at your code.

## TL;DR

- **0.0.x** emphasized newtype wrappers (`PositiveF64`, `NonNegativeI32`,
  `Point3`, `Vec3`, `Matrix4x3`, ...) and a trait + proc-macro system for
  callbacks.
- **0.1+** uses plain types (`f64`, `i32`, `[f64; 3]`, `[f64; 12]`) and
  closures for callbacks. The API mirrors the underlying manifold3d C API
  directly, so users transitioning from C/C++ find parameter names and
  order where they expect.
- The `manifold3d-macros` crate is removed. Callbacks are now closures.
- Most method names changed: `new_cuboid` → `cube`, `convex_hull` → `hull`,
  `vertex_count` → `num_vert`, etc. The new names match the manifold3d C
  API (`manifold_cube`, `manifold_hull`, `manifold_num_vert`).

## Cargo.toml changes

```diff
 [dependencies]
-manifold3d = "0.0.6"
-manifold3d-macros = "0.0.3"  # remove
+manifold3d = "0.1"
```

### Feature flags

| 0.0.6 feature | 0.1+ equivalent |
|---|---|
| `parallel` | `parallel` (same, enabled by default) |
| `nalgebra_interop` | `nalgebra` |
| `static` | (no equivalent — always static) |
| `export` | (no equivalent — upstream removed MeshIO in v3.4.0; use a dedicated mesh I/O crate like `stl_io`, `tobj`, or `gltf`) |

## Type changes

### Newtype wrappers → plain primitives

0.0.6 had validated newtypes; 0.1+ accepts plain primitives and validates
at the C kernel boundary.

| 0.0.6 | 0.1+ |
|---|---|
| `PositiveF64` | `f64` |
| `NonNegativeF64` | `f64` |
| `PositiveI32` | `i32` |
| `NonNegativeI32` | `i32` |
| `NormalizedAngle` | `f64` (degrees) |
| `Point3` / `Vec3` | `[f64; 3]` |
| `Point2` / `Vec2` | `[f64; 2]` |
| `Matrix4x3` | `[f64; 12]` (column-major) |

```rust
// Before
let cube = Manifold::new_cuboid(
    PositiveF64::new(10.0).unwrap(),
    PositiveF64::new(10.0).unwrap(),
    PositiveF64::new(10.0).unwrap(),
    true,
);

// After
let cube = Manifold::cube(10.0, 10.0, 10.0, true);
```

```rust
// Before
cube.translate(Vec3::new(1.0, 2.0, 3.0));

// After
cube.translate(1.0, 2.0, 3.0);
```

## Method renames

### `Manifold`

| 0.0.6 | 0.1+ |
|---|---|
| `Manifold::new_empty()` | `Manifold::empty()` |
| `Manifold::new_tetrahedron()` | `Manifold::tetrahedron()` |
| `Manifold::new_cuboid(x, y, z, center)` | `Manifold::cube(x, y, z, center)` |
| `Manifold::new_cylinder(h, rl, rh, segs, center)` | `Manifold::cylinder(h, rl, rh, segs, center)` |
| `Manifold::new_sphere(r, segs)` | `Manifold::sphere(r, segs)` |
| `Manifold::from_mesh_gl(&mesh)` | `Manifold::from_mesh_f64(&verts, n_props, &tris)` or `from_mesh_f32` |
| `Manifold::compose_from_vec(&vec)` | `Manifold::compose(&[manifolds])` |
| `Manifold::convex_hull_from_points(&points)` | `Manifold::hull_pts(&points)` |
| `Manifold::extrude_polygons(...)` | `Manifold::extrude(&cross_section, height)` or `CrossSection::extrude(height)` |
| `Manifold::revolve_polygons(...)` | `Manifold::revolve(&cross_section, segments, degrees)` |
| `.convex_hull()` | `.hull()` |
| `.batch_convex_hull(&others)` | `Manifold::batch_hull(&[manifolds])` |
| `.vertex_count()` | `.num_vert()` |
| `.edge_count()` | `.num_edge()` |
| `.triangle_count()` | `.num_tri()` |
| `.properties_per_vertex_count()` | `.num_prop()` |
| `.last_operation_status()` | (removed; check status through `CsgError` returned by fallible constructors) |
| `.slice_by_height(h)` | `.slice_at_z(h)` returns `Vec<Vec<[f64; 2]>>`, or `.slice_to_cross_section(h)` returns `CrossSection` |
| `.refine_via_edge_splits(n)` | `.refine(n)` (takes `i32`) |
| `.refine_to_edge_length(len)` | `.refine_to_length(len)` |
| `.split_by_offset_plane(op)` | `.split_by_plane([nx, ny, nz], offset)` |
| `.trim_by_offset_plane(op)` | `.trim_by_plane([nx, ny, nz], offset)` |
| `.mirror(plane)` | `.mirror([nx, ny, nz])` |
| `.rotate(Rotation::new(...))` | `.rotate(x_deg, y_deg, z_deg)` |
| `.minimum_gap(&other, len)` | `.min_gap(&other, len)` |
| `.replace_vertex_properties(...)` (trait-based) | `.set_properties(num_prop, |new, pos, old| { ... })` (closure) |
| `.as_mesh()` | `.to_mesh_f64()` returns `(Vec<f64>, usize, Vec<u64>)` or `.to_mesh_f32()` |

### Boolean operations

0.0.6 had `BooleanOperation` enum; 0.1+ uses `OpType` (re-exported from the
sys crate) and adds operator overloads.

```rust
// Before
let result = a.boolean(&b, BooleanOperation::Union);
// or
let result = a.union(&b);

// After — any of these work:
let result = a.boolean(&b, OpType::Add);
let result = a.union(&b);
let result = &a + &b;           // operator overload
let result = &a - &b;           // difference
let result = &a ^ &b;           // intersection (^, since & is borrow)
```

### `BoundingBox`

| 0.0.6 | 0.1+ |
|---|---|
| `BoundingBox::new(Point3, Point3)` | `BoundingBox::new([f64; 3], [f64; 3])` |
| `.min_point()` / `.max_point()` | `.min()` / `.max()` |
| `.expand_to_include_point(p)` | `.include_point(p)` |
| `.multiply(scale)` | `.mul(scale)` |
| `.contains_bounding_box(&other)` | `.contains_box(&other)` |
| `.overlaps_bounding_box(&other)` | `.overlaps_box(&other)` |

### `CrossSection`

`CrossSection` in 0.1+ is a first-class type (not just accessed via
`Polygons::cross_section`). Full method set: `square`, `circle`,
`from_polygons`, boolean ops, `offset`, `hull`, `transform`, `extrude`, etc.

```rust
// Before
let polys = Polygons::from_simple_polygons(vec![...]);
let cs = polys.cross_section(FillRule::EvenOdd);

// After
let cs = CrossSection::from_polygons(&[vec![[0.0, 0.0], [1.0, 0.0], ...]]);
// or
let cs = CrossSection::square(10.0, 10.0, true);
```

### Global quality settings

```rust
// Before
set_min_circular_angle(NormalizedAngle::new(...));
set_circular_segments(PositiveI32::new(32).unwrap());

// After
manifold_csg::set_min_circular_angle(10.0);
manifold_csg::set_circular_segments(32);
```

## Callbacks: warp and set_properties

This is the biggest semantic change. 0.0.6 used a trait-based system with
proc macros; 0.1+ uses plain closures.

### Warp

```rust
// Before (0.0.6)
use manifold3d::macros::manifold::warp;

#[warp]
struct MyWarp;

impl WarpVertex for MyWarp {
    fn warp_vertex(&self, point: Point3) -> Point3 {
        Point3::new(point.x + 1.0, point.y, point.z)
    }
}

let warped = manifold.warp(Pin::new(&MyWarp));
```

```rust
// After (0.1+)
let warped = manifold.warp(|x, y, z| [x + 1.0, y, z]);
```

### Set properties

```rust
// Before (0.0.6)
use manifold3d::macros::manifold::manage_vertex_properties;

#[manage_vertex_properties]
struct MyReplacer;

impl ReplaceVertexProperties for MyReplacer {
    type Ctx = ();
    fn replace_vertex_properties(
        &self,
        _ctx: &mut Self::Ctx,
        position: Vec3,
        _old: &[f64],
        new: &mut [f64],
    ) {
        new[0] = position.x;
    }
}

let result = manifold.replace_vertex_properties(&MyReplacer, &mut (), 3);
```

```rust
// After (0.1+)
let result = manifold.set_properties(3, |new, pos, _old| {
    new[0] = pos[0];
});
```

## Removed features

- **`manifold3d-macros`** crate — no longer needed; callbacks are closures.
- **`export` feature** — upstream removed `MeshIO` from their public API in
  v3.4.0. Use a dedicated mesh I/O crate (see feature flag table above).
- **`static` feature** — always static now (no behavior change; just no
  dynamic linking option).
- **Newtype validation errors** — `PositiveF64::new()`, `EdgeSplitCount::new()`,
  etc. returned `Result`. In 0.1+, invalid values are caught at the
  C kernel boundary and reported via `CsgError::ManifoldStatus`.

## Sys crate (`manifold3d-sys`)

If you were using `manifold3d-sys` directly: the API is roughly compatible
at the C signature level, but:
- Version jumps from `0.0.7` to `3.4.x` (new scheme where major.minor tracks
  upstream manifold3d version).
- No `export` or `static` features.
- Always builds from source with cmake (no prebuilt option).

## AI-assisted migration

Paste this prompt into an AI assistant along with your code:

> I have Rust code using `manifold3d = "0.0.6"`. Please migrate it to
> `manifold3d = "0.1"` using the migration guide at
> https://github.com/zmerlynn/manifold-csg/blob/main/MIGRATION_FROM_0.0.6.md
>
> Key rules:
> 1. Replace newtype wrappers (`PositiveF64`, `NormalizedAngle`, `Point3`,
>    `Vec3`, `Matrix4x3`) with plain primitives (`f64`, `[f64; 3]`, `[f64; 12]`).
> 2. Rename methods per the renames table (e.g., `new_cuboid` → `cube`,
>    `convex_hull` → `hull`, `vertex_count` → `num_vert`).
> 3. Convert `#[warp]` / `#[manage_vertex_properties]` trait impls to
>    closures passed to `.warp()` / `.set_properties()`.
> 4. Remove the `manifold3d-macros` dependency from Cargo.toml.
> 5. Replace `nalgebra_interop` feature with `nalgebra`.
> 6. Flag any use of the `export` feature — there's no direct replacement;
>    suggest a dedicated mesh I/O crate.

## Getting help

File issues at https://github.com/zmerlynn/manifold-csg/issues with the
`migration` label.
