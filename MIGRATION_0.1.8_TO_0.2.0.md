# Migration guide: `manifold-csg` 0.1.8 → 0.2.0

Two breaking changes, both driven by upstream manifold v3.5.0:

1. `ExecutionContext` attachment moved from an explicit-parameter model
   to an attach-then-eager-op model.
2. `MeshGL::update_normals` / `MeshGL64::update_normals` were removed
   upstream; the same effect is now produced by
   `Manifold::get_meshgl_w_normals` / `get_meshgl64_w_normals` at
   extraction time.

If you used neither, no code changes are needed; just bump the dependency.

**This guide is structured so an AI coding assistant can read it and migrate
code automatically.** If you have a Claude/Copilot/Cursor-style tool, point
it at this file and at your code. AI prompt at the end.

## Cargo.toml

```diff
 [dependencies]
-manifold-csg = "0.1.8"
+manifold-csg = "0.2.0"
```

Or for the facade:

```diff
 [dependencies]
-manifold3d = "0.1.8"
+manifold3d = "0.2.0"
```

## `Manifold::status_with_context` removed

The single method `m.status_with_context(&ctx)` is split into two: attach
the context to the manifold via `with_context`, then trigger eager
evaluation via `status` (newly public, was internal in 0.1.x). Mechanical
rewrite:

```diff
 let ctx = ExecutionContext::new();
-let status = manifold.status_with_context(&ctx);
+let status = manifold.with_context(&ctx).status();
```

The new model lets you attach a context once and run several eager ops
(`status`, `refine*`, mesh extraction) under it, instead of passing it to a
single specific status call. Deferred ops (booleans, transforms, batch
ops) ignore any attached context and produce no-context results, so attach
late if you want a context to apply.

## `MeshGL::update_normals` / `MeshGL64::update_normals` removed

Upstream removed the in-place normal-recomputation entry points on
`MeshGL`. The replacement depends on what produced the mesh.

### If you're starting from a `Manifold`

Call `Manifold::calculate_normals(normal_idx, min_sharp_angle)` to bake
normals into the manifold itself, then extract:

```diff
-let mut mesh = manifold.to_meshgl64();
-mesh.update_normals(3);
+let mesh = manifold.calculate_normals(3, 60.0).to_meshgl64();
```

`min_sharp_angle` is the dihedral-angle threshold (degrees) above which an
edge is treated as a hard crease; `60.0` is a common default. This works
regardless of whether the manifold is an "original" (a leaf imported via
`from_mesh_*`) or the result of CSG.

`get_meshgl_w_normals(normal_idx)` / `get_meshgl64_w_normals(normal_idx)`
also exist, but they only recompute normals for *non-original* manifolds.
Use `calculate_normals` if you need the operation to be unconditional.

### If you only have a free-floating `MeshGL` / `MeshGL64`

There is no direct replacement. Wrap the mesh in a `Manifold` first
(`Manifold::from_mesh_f64(...)`), then follow the recipe above.

## AI prompt

> I have Rust code using `manifold-csg = "0.1.x"` (or `manifold3d = "0.1.x"`).
> Please migrate it to the `0.2.0` line using the migration guide at
> https://github.com/zmerlynn/manifold-csg/blob/main/MIGRATION_0.1.8_TO_0.2.0.md
>
> Key rules:
> 1. Bump the Cargo.toml dependency from `0.1.x` to `0.2.0`.
> 2. Rewrite every `m.status_with_context(&ctx)` as
>    `m.with_context(&ctx).status()`.
> 3. Replace `MeshGL::update_normals(n)` / `MeshGL64::update_normals(n)`
>    calls. If the mesh was produced from a `Manifold`, rewrite the path
>    that produced it as
>    `manifold.calculate_normals(n, 60.0).to_meshgl64()` (or
>    `to_meshgl_f32()` for the f32 variant). Do NOT suggest
>    `get_meshgl_w_normals` as a 1:1 replacement: that path skips normal
>    recomputation when the manifold is an "original" (leaf imported via
>    `from_mesh_*`). If the mesh did not come from a `Manifold`, flag the
>    call as having no direct replacement and suggest wrapping the mesh
>    in a `Manifold` first via `Manifold::from_mesh_f64`.
