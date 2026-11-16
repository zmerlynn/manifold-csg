# Migrating from manifold-csg 0.4.2 to 0.5.0

**Status: in progress.** 0.5.0 tracks upstream manifold master on its way to
4.0 and is built on the `next` branch. It is not published yet, and this guide
grows with each breaking change that lands there.

Upstream's 4.0 work changes the C API under this crate (handles are no longer
caller-allocated) and removes a few features outright. Most of that stays
inside the crate. What reaches your code is listed below: a few removed
methods, one changed signature, and several changes in behavior that
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

## Behavioral: `Manifold::compose` is a union

Upstream turned `Compose` into `BatchBoolean(Add)` and deprecated it, so
overlapping inputs now merge instead of overlapping. For disjoint inputs the
result is the same. Use `batch_union`, which says what it does:

```diff
-let both = Manifold::compose(&[a, b]);
+let both = Manifold::batch_union(&[a, b]);
```

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

- `smooth_by_normals(normal_idx)` for smoothing driven by vertex normals
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
**`reset_to_circular_defaults()` puts it back to `1e-12`**, because upstream
resets every quality global there, so set the precision after any reset.
`MeshGL::tolerance` and `MeshGL64::tolerance` are unchanged.

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

`RayHit` has a new `crossing` field (`+1` where the ray enters the solid, `-1`
where it leaves) and is `#[non_exhaustive]`, so a struct literal no longer
compiles. Results only come from `Manifold::ray_cast`, so this only matters if
you built `RayHit` values yourself, for example in tests.

## New

- `Manifold::winding_number(point)`.
- `set_relative_precision` / `get_relative_precision` (see above).
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
- `ManifoldRayHit` has a new `crossing` field, which changes its layout.

## AI prompt

If you would rather hand this to an assistant, the following is enough context:

> I am upgrading the Rust crate `manifold-csg` from 0.4.2 to 0.5.0 (or the
> `manifold3d` facade, same versions). Please update my `Cargo.toml` to
> `"0.5"`, then make these changes:
> - `Manifold::as_original()` now takes an `id`: call `as_original(-1)` to
>   keep the old behavior.
> - Replace `Manifold::compose(..)` with `Manifold::batch_union(..)`, and
>   `CrossSection::compose(..)` with `CrossSection::batch_union(..)`.
> - `Manifold::smooth_f32`, `smooth_f64`, `smooth_f64_with_context` and
>   `smooth_out` are gone. Use `smooth_by_normals` (after
>   `calculate_normals` if needed), or pass halfedge tangents through
>   `MeshGL64::new_with_tangents` and `refine`. Ask me if the intent is
>   unclear.
> - `Manifold::set_tolerance(t)` is gone. Use the global
>   `manifold_csg::set_relative_precision(t)` once, and remove
>   `get_tolerance` calls.
> - `FillRule::Negative` is gone: reverse each ring and use
>   `FillRule::Positive`.
> - `RayHit` is `#[non_exhaustive]` with a new `crossing` field, so replace
>   any `RayHit { .. }` literals.
> Also check code that reads mesh runs (`num_run`, `run_original_id`,
> `backside`) after a boolean: primitives now share original ID 0, so call
> `.as_original(-1)` on each input that should get its own run. Check
> tests that pin vertex counts after `Manifold::simplify` (upstream's
> decimation changed), calls to `Manifold::revolve` with a zero or negative
> angle (zero is now an error, negative revolves the other way), and any
> `reset_to_circular_defaults()` after `set_relative_precision` (the reset
> now clears the precision too).
