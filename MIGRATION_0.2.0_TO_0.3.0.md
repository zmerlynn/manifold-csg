# Migration guide: `manifold-csg` 0.2.0 -> 0.3.0

This release tightens the safe Rust API around status handling, mesh
construction, and upstream enum/default behavior.

Breaking changes:

1. `Manifold::status()` now returns `Result<(), CsgError>`.
2. `MeshGL::new`, `MeshGL64::new`, and their tangent constructors now return
   `Result<_, CsgError>`.
3. Empty mesh input is valid. `CsgError::EmptyMesh` was removed.
4. `CrossSection::from_polygons()` now uses upstream's default
   `FillRule::Positive` instead of `FillRule::EvenOdd`.
5. `CrossSection::from_simple_polygon()` now uses `FillRule::Positive`; the
   fill-rule-taking form was renamed to `from_simple_polygon_with_fill_rule()`.
6. `manifold_csg::OpType` is now a safe crate enum instead of a re-export of
   `manifold_csg_sys::ManifoldOpType`.
7. Safe public enums intended to follow upstream growth are now
   `#[non_exhaustive]`; raw `ManifoldError` is also non-exhaustive.
8. Panics in user callbacks now propagate after the FFI call returns safely,
   instead of being swallowed by the Rust trampoline.

Additions:

1. `Manifold::raw_status()` returns the raw upstream `ManifoldError`.
2. `Manifold::to_meshgl()` and `Manifold::to_meshgl64()` return the full mesh
   containers, including metadata.
3. `Manifold::to_meshgl_with_normals()` and `to_meshgl64_with_normals()` return
   full mesh containers with normals baked into vertex properties.
4. `MeshGL::new_with_options()` and `MeshGL64::new_with_options()` construct
   meshes with run, original-ID, merge, and tangent metadata.
5. `Manifold::from_meshgl()` and `from_meshgl64()` construct manifolds from
   mesh containers, including containers built with metadata options.
6. `Default` for `Manifold` and `CrossSection` returns empty geometry.
7. `manifold-csg-sys` / `manifold3d-sys` move to `3.5.101`, tracking the same
   upstream `3.5.x` source plus binding fixes.

If you do not construct meshes directly, call `status()`, match public enums
exhaustively, depend on `from_polygons()` using EvenOdd semantics, or call
`from_simple_polygon()` with a fill rule, most code only needs a dependency
bump.

**This guide is structured so an AI coding assistant can read it and migrate
code automatically.** If you have a Claude/Copilot/Cursor-style tool, point it
at this file and at your code. AI prompt at the end.

## Cargo.toml

```diff
 [dependencies]
-manifold-csg = "0.2.0"
+manifold-csg = "0.3.0"
```

Or for the facade:

```diff
 [dependencies]
-manifold3d = "0.2.0"
+manifold3d = "0.3.0"
```

## `Manifold::status()` returns `Result`

Use `?`, `is_ok()`, or match on `Result`.

```diff
-use manifold_csg_sys::ManifoldError;
-
 let result = manifold.with_context(&ctx).status();
-assert_eq!(result, ManifoldError::NoError);
+assert!(result.is_ok());
```

If you need the raw upstream status enum:

```diff
-let status = manifold.status();
+let status = manifold.raw_status();
```

## Mesh constructors return `Result`

`MeshGL` constructors now validate buffer shape before calling the C API.

```diff
-let mesh = MeshGL64::new(&verts, n_props, &indices);
+let mesh = MeshGL64::new(&verts, n_props, &indices)?;
```

Empty meshes are valid:

```rust
let mesh = MeshGL64::new(&[], 3, &[])?;
let manifold = Manifold::from_mesh_f64(&[], 3, &[])?;
assert!(manifold.is_empty());
```

Shape errors are now reported as `CsgError::InvalidInput`, for example
`n_props < 3`, vertex-property length not divisible by `n_props`, triangle
index length not divisible by 3, or tangent length not equal to
`num_tri * 3 * 4`.

## `from_polygons()` uses `FillRule::Positive`

This matches upstream C++/Python defaults. If you relied on EvenOdd behavior,
call the explicit constructor. This also affects
`Manifold::slice_to_cross_section()`, which reconstructs a cross-section from
sliced polygons using the same upstream default.

```diff
-let cs = CrossSection::from_polygons(&polygons);
+let cs = CrossSection::from_polygons_with_fill_rule(&polygons, FillRule::EvenOdd);
```

`FillRule::Positive` treats positively oriented contours as filled regions and
negatively oriented contours as holes.

## `from_simple_polygon()` uses `FillRule::Positive`

The old `from_simple_polygon(points, fill_rule)` spelling moved to
`from_simple_polygon_with_fill_rule(points, fill_rule)`.

```diff
-let cs = CrossSection::from_simple_polygon(&points, FillRule::EvenOdd);
+let cs = CrossSection::from_simple_polygon_with_fill_rule(&points, FillRule::EvenOdd);
```

For upstream default behavior:

```diff
-let cs = CrossSection::from_simple_polygon(&points, FillRule::Positive);
+let cs = CrossSection::from_simple_polygon(&points);
```

## `OpType` is now a safe enum

Most code that imports `manifold_csg::OpType` keeps the same variant names. Code
that explicitly names the raw sys type should use the safe type for safe API
calls:

```diff
-let op = manifold_csg_sys::ManifoldOpType::Add;
+let op = manifold_csg::OpType::Add;
 let result = a.boolean(&b, op);
```

## Public enums are non-exhaustive

Downstream exhaustive matches need a wildcard arm.

```diff
 match fill_rule {
     FillRule::EvenOdd => { /* ... */ }
     FillRule::NonZero => { /* ... */ }
     FillRule::Positive => { /* ... */ }
     FillRule::Negative => { /* ... */ }
+    _ => { /* future upstream variant */ }
 }
```

This applies to `FillRule`, `JoinType`, `OpType`, `CsgError`, and raw
`manifold_csg_sys::ManifoldError`. Other raw `manifold-csg-sys` enums remain
exhaustive for the bundled upstream source.

## Raw sys changes

`manifold-csg-sys` / `manifold3d-sys` `3.5.101` add the upstream
`ManifoldError::InvalidTangents` and `ManifoldError::Cancelled` variants and
mark `ManifoldError` as `#[non_exhaustive]`. Downstream matches over
`ManifoldError` must add a wildcard arm.

This is a deliberate binding-correction exception to the usual sys patch-bump
rule: bundled manifold3d 3.5.0 can already return these status codes, so the
old incomplete enum could construct invalid Rust enum values through safe
status queries. Making `ManifoldError` non-exhaustive means this release has
one explicit sys compatibility break, but future upstream status additions can
be represented without breaking downstream exhaustive matches again.

The same sys patch release also changes the public
`ManifoldMeshGLOptions` / `ManifoldMeshGL64Options` input pointer fields from
`*mut` to `*const`. The upstream C header spells these input-only pointers as
mutable, but the Rust binding uses const pointers for the same ABI so safe
wrappers can pass shared slices without casting away constness.

## Callback panics now propagate

`Manifold::warp()`, `Manifold::set_properties()`, `Manifold::from_sdf()`,
`Manifold::from_sdf_seq()`, and `CrossSection::warp()` now catch user callback
panics before they cross the C stack, clean up any partially created handle,
then resume the original panic payload on the Rust side.

Previously, these callback panics were swallowed by sentinel return values or
ignored outputs. Code should not rely on those panics being converted into
identity warps, infinite SDF samples, or zeroed property buffers.

## Full mesh extraction

Use `to_mesh_f32()` / `to_mesh_f64()` when you only need the tuple. Use the new
container-returning methods when you need metadata:

```rust
let mesh = manifold.to_meshgl64();
let run_ids = mesh.run_original_id();
let face_ids = mesh.face_id();

let mesh_with_normals = manifold.to_meshgl64_with_normals(3);
```

## Mesh construction with metadata

Use `MeshGLOptions` / `MeshGL64Options` with `new_with_options()` when importing
mesh data that already has run IDs, merge vectors, or halfedge tangents:

```rust
let mesh = MeshGL64::new_with_options(
    &verts,
    3,
    &triangles,
    MeshGL64Options::new()
        .runs(&run_indices, &run_original_ids)
        .merge_vertices(&merge_from, &merge_to)
        .halfedge_tangents(&halfedge_tangents),
)?;
let manifold = Manifold::from_meshgl64(&mesh)?;
```

The current C shim exposes this subset of upstream `MeshGL` input metadata.
Face IDs, run transforms, run flags, and tolerance are still output-only from
the safe wrapper until the C shim exposes them for construction.

## AI prompt

> I have Rust code using `manifold-csg = "0.2.x"` (or `manifold3d = "0.2.x"`).
> Please migrate it to the `0.3.0` line using the migration guide at
> https://github.com/zmerlynn/manifold-csg/blob/main/MIGRATION_0.2.0_TO_0.3.0.md
>
> Key rules:
> 1. Bump the Cargo.toml dependency from `0.2.x` to `0.3.0`.
> 2. Rewrite `m.status()` callers for `Result<(), CsgError>`. Use
>    `m.raw_status()` only when code truly needs `ManifoldError`.
> 3. Add `?`, `.expect(...)`, or explicit error handling for `MeshGL::new`,
>    `MeshGL64::new`, and their `new_with_tangents` variants.
> 4. Replace uses of `CsgError::EmptyMesh`; empty meshes are valid now.
> 5. If code relied on EvenOdd polygon filling, rewrite
>    `CrossSection::from_polygons(&polys)` to
>    `CrossSection::from_polygons_with_fill_rule(&polys, FillRule::EvenOdd)`.
>    Audit `slice_to_cross_section()` uses for the same Positive fill-rule
>    behavior.
> 6. Rewrite `CrossSection::from_simple_polygon(&points, rule)` to
>    `CrossSection::from_simple_polygon_with_fill_rule(&points, rule)`.
> 7. Use `manifold_csg::OpType` instead of `manifold_csg_sys::ManifoldOpType`
>    with safe `boolean` APIs.
> 8. Add wildcard arms to exhaustive matches over safe public manifold enum
>    types and `manifold_csg_sys::ManifoldError`.
> 9. Do not rely on panics in `warp`, `set_properties`, `from_sdf`,
>    `from_sdf_seq`, or cross-section `warp` callbacks being swallowed; they
>    now propagate after FFI cleanup.
