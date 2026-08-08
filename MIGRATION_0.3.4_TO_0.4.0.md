# Migrating from manifold-csg 0.3.4 to 0.4.0

**If you do not use the `nalgebra` feature, there is nothing to do.** Bump the
version and you are done. No API changed in this release.

If you do use it, the optional `nalgebra` dependency moved from `0.34` to
`0.35`, and you need to move with it.

## Why it matters

`nalgebra::Vector3<f64>` and `nalgebra::Point3<f64>` appear directly in this
crate's public signatures:

```rust
pub fn trim_by_plane_nalgebra(&self, normal: &nalgebra::Vector3<f64>, offset: f64) -> Self
pub fn mirror_nalgebra(&self, normal: &nalgebra::Vector3<f64>) -> Self
pub fn bounding_box_nalgebra(&self) -> Option<(nalgebra::Point3<f64>, nalgebra::Point3<f64>)>
pub fn to_vertices_and_faces(&self) -> (Vec<nalgebra::Point3<f64>>, Vec<[u32; 3]>)
```

Cargo treats `nalgebra 0.34` and `nalgebra 0.35` as incompatible, so if your
project still requires `0.34`, both end up in your dependency tree. Your
`Vector3<f64>` is then a *different type* from the one these methods take, and
the calls stop compiling with a "mismatched types" error naming what look like
two identical types.

## Cargo.toml

```diff
 [dependencies]
-manifold-csg = { version = "0.3", features = ["nalgebra"] }
-nalgebra = "0.34"
+manifold-csg = { version = "0.4", features = ["nalgebra"] }
+nalgebra = "0.35"
```

Using the `manifold3d` facade instead:

```diff
-manifold3d = { version = "0.3", features = ["nalgebra"] }
-nalgebra = "0.34"
+manifold3d = { version = "0.4", features = ["nalgebra"] }
+nalgebra = "0.35"
```

## Code changes

None on this crate's side. Any changes you need are nalgebra's own 0.34 to
0.35 changes; see [nalgebra's
changelog](https://github.com/dimforge/nalgebra/blob/main/CHANGELOG.md).

## Rust version

nalgebra 0.35 requires Rust 1.89, up from 0.34's 1.87. This crate's own
`rust-version` stays `1.85`, which remains the floor when the `nalgebra`
feature is off.

## Checking

If two nalgebra versions are still resolving, this shows it:

```sh
cargo tree -d | grep -A2 nalgebra
```

## AI prompt

If you would rather hand this to an assistant, the following is enough context:

> I am upgrading the Rust crate `manifold-csg` from 0.3.4 to 0.4.0 (or the
> `manifold3d` facade, same versions). The only change is that its optional
> `nalgebra` feature now requires nalgebra 0.35 instead of 0.34. Please update
> my `Cargo.toml` so `manifold-csg` (or `manifold3d`) is `"0.4"` and my own
> `nalgebra` dependency is `"0.35"`, then fix any compile errors that come from
> nalgebra's own 0.34-to-0.35 changes. No manifold-csg API changed, so any
> error mentioning two seemingly identical `Vector3<f64>` or `Point3<f64>`
> types means a stale nalgebra version is still resolving somewhere; check with
> `cargo tree -d`.
