# Migrating from manifold-csg 0.4.2 to 0.5.0

**Status: in progress.** 0.5.0 tracks upstream manifold master on its way to
4.0 and is built on the `next` branch. It is not published yet, and this guide
grows with each breaking change that lands there.

Upstream's 4.0 work changes the C API under this crate (handles are no longer
caller-allocated) and removes a few features outright. Most of that stays
inside the crate. What reaches your code is listed below: some removed
methods, a few changed signatures, and several changes in behavior that
compile cleanly (marked **Behavioral**), so read those even if your build
passes.

## Cargo.toml

```diff
 [dependencies]
-manifold-csg = "0.4"
+manifold-csg = "0.5"
```

Using the `manifold3d` facade instead:

```diff
-manifold3d = "0.4"
+manifold3d = "0.5"
```

## `as_original` takes an ID

```diff
-let m = part.as_original();
+let m = part.as_original(-1);
```

`-1` (any negative value) keeps the old behavior: a new unique ID. `0` matches
all default-constructed manifolds, which skips tracking the joints between
inputs.

## Behavioral: primitives start with original ID 0

`cube`, `sphere`, `cylinder` and the other primitives used to get a unique
original ID each. They now all start at 0, so after a boolean their faces
share one run and coplanar faces can merge across inputs. If you read runs
back out of a `MeshGL` (`num_run`, `run_original_id`, `backside`), mark each
input as an original first:

```diff
-let m = cube.difference(&sphere).to_meshgl64();
+let m = cube.as_original(-1).difference(&sphere.as_original(-1)).to_meshgl64();
```

## `Manifold::compose` is gone

Upstream first made `Compose` a union and then removed it. Use `batch_union`,
which gives the same result for disjoint inputs and merges overlapping ones:

```diff
-let both = Manifold::compose(&[a, b]);
+let both = Manifold::batch_union(&[a, b]);
```

## Circular quality is per call

The global setters are gone: `set_circular_segments`, `set_min_circular_angle`,
`set_min_circular_edge_length`, `reset_to_circular_defaults` and
`get_circular_segments`. Instead, `Manifold::sphere`, `cylinder`, `revolve`,
`CrossSection::circle` and `offset` take a `CircularQuality`, or a plain segment
count as before. Calls that pass a count compile unchanged, and except for a
partial `revolve` (below) they mean what they did:

```rust
let a = Manifold::sphere(1.0, 32); // 32 segments, as before
let b = Manifold::sphere(1.0, 0); // default quality, as before
```

Code that changed the globals passes the quality to each call instead:

```diff
-manifold_csg::set_min_circular_edge_length(0.1);
-let s = Manifold::sphere(1.0, 0);
+let fine = CircularQuality { min_edge_length: 0.1, ..Default::default() };
+let s = Manifold::sphere(1.0, fine);
```

```diff
-let n = manifold_csg::get_circular_segments(r);
+let n = CircularQuality::default().segments_for(r);
```

`segments: 0` means the default quality, as `0` did before, but it no longer
reads any global state, so a quality setting from elsewhere in the process
cannot change your geometry.

## Behavioral: partial `revolve` counts per full circle

A segment count passed to `revolve` used to be the number of divisions across
the swept angle. It is now the count for a full circle, and a partial sweep
gets the matching fraction, rounded up. So `revolve(&cs, 16, 90.0)` used to
build 16 divisions and now builds 4. Use `CircularQuality::for_divisions` to
ask for a number of divisions across the arc:

```diff
-let m = Manifold::revolve(&cs, 16, 90.0);
+let m = Manifold::revolve(&cs, CircularQuality::for_divisions(16, 90.0), 90.0);
```

Every full circle has at least 3 segments, so a sweep over 120 degrees gets
at least 2 divisions and one over 240 degrees at least 3. A partial revolve at
the default quality also changed slightly: its division count is now rounded
up instead of down. A full 360-degree revolve is unchanged.

## Behavioral: normals live in channels 3-5

`calculate_normals` and `smooth_by_normals` no longer take a property index.
Upstream always puts normals in MeshGL property channels 3-5, right after
position:

```diff
-let m = m.calculate_normals(3, 60.0).smooth_by_normals(3);
+let m = m.calculate_normals(60.0).smooth_by_normals();
```

The old index counted properties after position, so `calculate_normals(3, ..)`
put normals in MeshGL channels 6-8 (and `num_prop()` became 6). They are now in
channels 3-5 (and `num_prop()` is 3). Code that reads normals out of
`vert_properties` at a fixed offset, or that stored other data in channels 3-5,
has to change. An old index of 0 gives the same layout as before.

`to_meshgl_with_normals`, `to_meshgl64_with_normals`, `to_mesh_f32_with_normals`
and `to_mesh_f64_with_normals` are gone. Call `calculate_normals` and then the
plain getter, which now returns the normals in world space after transforms
and booleans:

```diff
-let (verts, n_props, tris) = m.to_mesh_f64_with_normals(3);
+let (verts, n_props, tris) = m.calculate_normals(60.0).to_mesh_f64();
```

That rewrite recomputes the normals. If the mesh brought its own normals in
channels 3-5, keep them by marking the runs as normals when you build the mesh
instead: set bit 1 in `run_flags` on `MeshGL64Options` (or `MeshGLOptions`),
one byte per run. Upstream then rotates them along with transforms. Without the
flag, imported normals are carried through as plain properties, so a rotation
leaves them pointing the old way.


## Behavioral: `simplify` keeps different vertices

`Manifold::simplify` uses upstream's new decimation. Surfaces still move by
less than the tolerance, but the same tolerance keeps a different subset of
vertices, usually fewer. Re-check tests that pin vertex or triangle counts
after `simplify`.

## Behavioral: `revolve` angle rules

`Manifold::revolve` now:

- treats a zero or non-finite `revolve_degrees` as invalid input, so
  `status()` returns an error (it used to return an empty manifold);
- revolves the other way for a negative angle (it used to give an empty
  result);
- builds a solid for small angles that used to give an empty result.

## Smoothing constructors are gone

Upstream removed the mesh-plus-sharpened-edges `Smooth` constructors and
`SmoothOut`, so these are gone: `Manifold::smooth_f32`, `smooth_f64`,
`smooth_f64_with_context` and `smooth_out`. Upstream's replacements:

- `smooth_by_normals()` for smoothing driven by vertex normals
  (compute them with `calculate_normals` first if the mesh has none).
- Halfedge tangents supplied directly, through
  `MeshGL64::new_with_tangents` / `new_with_options`, then `refine`.

## Per-manifold tolerance is gone

`Manifold::set_tolerance` and `get_tolerance` are gone. Upstream replaced them
with one process-global relative precision:

```diff
-let m = m.set_tolerance(1e-6);
+manifold_csg::set_relative_precision(1e-6);
```

The default is `1e-12` (double precision), and smaller values are clamped to
it. It affects every operation started afterwards, in every thread.

`MeshGL::tolerance` and `MeshGL64::tolerance` are gone too: a mesh no longer
carries a tolerance. `merge` takes it instead:

```diff
-let merged = mesh.merge();
+let merged = mesh.merge(0.0); // 0.0 = the bounding-box baseline, as before
```

A larger value welds open-edge vertices up to that far apart.

## Behavioral: `face_id` is empty unless the input had face IDs

`MeshGL::face_id` / `MeshGL64::face_id` used to be filled from upstream's
coplanar-face calculation when no input carried face IDs. Now they are empty
in that case. Input face IDs still carry through to the output.

## CrossSection

- `FillRule::Negative` is gone. Reverse each contour and use
  `FillRule::Positive` for the same result:

  ```rust
  let reversed: Vec<Vec<[f64; 2]>> =
      rings.iter().map(|r| r.iter().rev().copied().collect()).collect();
  let cs = CrossSection::from_polygons_with_fill_rule(&reversed, FillRule::Positive);
  ```

  `EvenOdd`, `NonZero` and `Positive` work as before.
- `CrossSection::compose` is gone. Use `CrossSection::batch_union`, which
  gives the same result for disjoint pieces.

## `RayHit`

`RayHit` has new `crossing` (`+1` where the ray enters the solid, `-1` where it
leaves) and `original_id` fields, and is `#[non_exhaustive]`, so a struct
literal no longer compiles. Results only come from `Manifold::ray_cast`, so
this only matters if you built `RayHit` values yourself, for example in tests.

**Behavioral:** `face_id` is now a `u32` holding the input mesh's face ID (as in
`MeshGL64::face_id`), not an internal triangle index, and `original_id` names the
input mesh. Both are 0 for a mesh that was not tracked or had no face IDs, as
for a plain primitive. Code that used `face_id` to index into the result's
triangles has to find the triangle another way.

## New

- `Manifold::winding_number(point)`.
- `set_relative_precision` / `get_relative_precision` (see above).
- `CircularQuality` (see above).
- `MeshGLOptions` / `MeshGL64Options` builders `run_transform`, `run_flags` and
  `face_id`.
- `manifold_version()`, the linked library's `[major, minor, patch]`.

## Raw sys changes

If you use `manifold-csg-sys` directly, every function changed shape:

- `manifold_alloc_*`, `manifold_destruct_*` and the `*_size` functions are
  gone. Constructors and operations return a new handle (`T*`) that you free
  with the matching `manifold_delete_*`.
- Inputs are `const`.
- Data getters (`manifold_meshgl_vert_properties` and friends,
  `manifold_triangulation_tri_verts`) return a `const T*` borrowed from the
  object, valid while it lives, instead of copying into your buffer.
- `ManifoldFillRule` is gone. Use the `of_*`, `even_odd_*` and `non_zero_*`
  cross-section constructors.
- `ManifoldRayHit` has new `crossing` and `original_id` fields, and `face_id`
  is a `u32`, which changes its layout.
- `ManifoldMeshGLOptions` / `ManifoldMeshGL64Options` start with a
  `struct_size` field (set it to `size_of` the struct, or upstream aborts) and
  gain `run_transform`, `run_flags` and `face_id`.
- The circular constructors and `manifold_cross_section_offset` take a
  `*const ManifoldCircularQuality` (null for the default) instead of a segment
  count. The global circular setters and `manifold_get_circular_segments` are
  gone. `manifold_circular_quality_segments_for` and
  `manifold_circular_quality_for_divisions` are new.
- `manifold_calculate_normals` and `manifold_smooth_by_normals` lost their
  normal index, and `manifold_get_meshgl_w_normals` /
  `manifold_get_meshgl64_w_normals` and `manifold_compose` are gone.
- `manifold_cross_section_warp_context` is now `manifold_cross_section_warp`.
- `ManifoldIVec3`, `ManifoldVec4` and `ManifoldProperties` are gone.
- `manifold_meshgl_merge` / `manifold_meshgl64_merge` take a `tolerance`, and
  `manifold_meshgl_tolerance` / `manifold_meshgl64_tolerance` are gone.

## AI prompt

If you would rather hand this to an assistant, the following is enough context:

> I am upgrading the Rust crate `manifold-csg` from 0.4.2 to 0.5.0 (or the
> `manifold3d` facade, same versions). Please update my `Cargo.toml` to
> `"0.5"`, then make these changes:
> - `Manifold::as_original()` now takes an `id`: call `as_original(-1)` to
>   keep the old behavior.
> - Replace `Manifold::compose(..)` with `Manifold::batch_union(..)`, and
>   `CrossSection::compose(..)` with `CrossSection::batch_union(..)`.
> - The global circular quality setters (`set_circular_segments`,
>   `set_min_circular_angle`, `set_min_circular_edge_length`,
>   `reset_to_circular_defaults`) are gone. Pass a
>   `CircularQuality { .. }` to `sphere`, `cylinder`, `revolve`,
>   `CrossSection::circle` and `offset` instead, where the code relied on
>   them. Plain segment counts still work. Replace
>   `get_circular_segments(r)` with
>   `CircularQuality::default().segments_for(r)`.
> - A segment count passed to `Manifold::revolve` with an angle below 360
>   used to be the number of divisions across that angle. It is now a
>   full-circle count. Rewrite `revolve(&cs, n, deg)` with `n > 2` and
>   `deg < 360` as
>   `revolve(&cs, CircularQuality::for_divisions(n, deg), deg)`.
> - Drop the property index from `calculate_normals(idx, angle)` and
>   `smooth_by_normals(idx)`. Normals are now always in MeshGL property
>   channels 3-5 (the old index counted after position, so an old index of
>   3 meant channels 6-8): fix any code that reads normals at a fixed
>   offset in `vert_properties`, and flag code that keeps other properties
>   in channels 3-5. Replace
>   `to_mesh*_with_normals(idx)` / `to_meshgl*_with_normals(idx)` with
>   `calculate_normals(angle)` followed by the plain getter. If the mesh
>   was built with its own normals in channels 3-5, do not recompute them:
>   set bit 1 in `run_flags` on the MeshGL options instead, and ask me if
>   unsure which case applies.
> - `Manifold::smooth_f32`, `smooth_f64`, `smooth_f64_with_context` and
>   `smooth_out` are gone. Use `smooth_by_normals` (after
>   `calculate_normals` if needed), or pass halfedge tangents through
>   `MeshGL64::new_with_tangents` and `refine`. Ask me if the intent is
>   unclear.
> - `Manifold::set_tolerance(t)` is gone. Use the global
>   `manifold_csg::set_relative_precision(t)` once, and remove
>   `get_tolerance` calls.
> - `MeshGL::merge()` / `MeshGL64::merge()` take a tolerance: use
>   `merge(0.0)` for the old behavior. Remove `MeshGL::tolerance()` /
>   `MeshGL64::tolerance()` calls.
> - `FillRule::Negative` is gone: reverse each ring and use
>   `FillRule::Positive`.
> - `RayHit` is `#[non_exhaustive]` with new `crossing` and `original_id`
>   fields, so replace any `RayHit { .. }` literals. Its `face_id` is now a
>   `u32` input face ID, not a triangle index: flag any code that indexes
>   triangles with it.
> Also check code that reads mesh runs (`num_run`, `run_original_id`,
> `backside`) after a boolean: primitives now share original ID 0, so call
> `.as_original(-1)` on each input that should get its own run. Check
> tests that pin vertex counts after `Manifold::simplify` (upstream's
> decimation changed), calls to `Manifold::revolve` with a zero or negative
> angle (zero is now an error, negative revolves the other way), and code
> that reads `face_id()` from a mesh whose input had no face IDs (it is now
> empty).
