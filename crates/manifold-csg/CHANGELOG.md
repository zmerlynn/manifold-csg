# Changelog

Condensed release notes for `manifold-csg`, shipped inside the crate so it is
visible from the vendored dependency (not just on GitHub). Full per-version
migration guides live at the repo root and are linked below.

Watch especially for **behavioral** changes marked below: these compile cleanly
but change output, so the compiler will not catch them for you.

## 0.4.0

**If you do not enable the `nalgebra` feature, this release changes nothing
for you.** No API changed, no behavior changed, and the manifold3d pin is the
same as 0.3.4. It is a minor bump only because of the dependency change below.
`nalgebra` is not a default feature, so unless you opted into it, upgrading is
a version-number change and nothing else.

Full guide: <https://github.com/zmerlynn/manifold-csg/blob/main/MIGRATION_0.3.4_TO_0.4.0.md>

For users of the `nalgebra` feature:

- **Dependency (compile-breaking):** the optional `nalgebra` dependency moved
  from `0.34` to `0.35`, and your own `nalgebra` has to move with it.
  `nalgebra::Vector3<f64>` and `Point3<f64>` appear in the
  `Manifold::*_nalgebra` signatures, so cargo has to resolve one shared copy.
  Stay on 0.34 and you get two, whereupon your vectors are a different type
  from the ones these methods take. The error says so directly ("there are
  multiple different versions of crate `nalgebra` in the dependency graph"),
  so it should not cost you any debugging. The fix is `nalgebra = "0.35"` in
  your own manifest.
- Enabling the feature now requires Rust 1.89 (nalgebra 0.35's floor, up from
  0.34's 1.87). The crate's own `rust-version` stays `1.85`, which remains the
  floor without the feature.

Unchanged for everyone: the manifold3d pin, so `manifold-csg-sys` stays at
3.5.104.

## 0.3.4

- Ships `LICENSE-APACHE` and `LICENSE-MIT` inside the crate. They lived only at
  the repo root, outside every crate directory, so no published version back to
  0.1.x actually carried the license text the `license` field points at.
- Renamed the `segments` parameter to `circular_segments` on
  `Manifold::cylinder`, `Manifold::sphere`, and `CrossSection::circle`, to
  match the C API and the sibling methods that already used that name. Rust
  has no named arguments, so this changes only docs and IDE hints, not calls.
- Declares `rust-version = "1.85"`. The workspace has stated this MSRV all
  along, but no crate inherited it, so it never reached the published
  metadata. Cargo can now report an actionable error (and pick MSRV-compatible
  dependencies) instead of failing mid-compile on an older toolchain.
- Documented `to_mesh_f32` and `to_mesh_f32_with_normals` as lossy. The kernel
  holds geometry in f64 and these narrow on the way out; only the f32 *input*
  path carried that warning before.
- Upstream pin moved to manifold3d v3.5.2. It drops deferred deallocation:
  large buffer frees used to be enqueued on a low-priority background TBB
  arena, and are now freed synchronously. Upstream made the change to address
  a downstream report from Blender. No C API change, so no source or API
  changes here. Carried by `manifold-csg-sys` 3.5.104.

## 0.3.3

- Build fix: link `libc++` (not `libstdc++`) on all Apple platforms, not just
  macOS. iOS / tvOS / watchOS / visionOS builds previously failed to link
  because Apple SDKs ship only libc++. Detection now keys on the target vendor
  (`CARGO_CFG_TARGET_VENDOR == "apple"`) rather than an OS allow-list. Carried
  by `manifold-csg-sys` 3.5.103; thanks to #52 for the original macOS-list fix.
  No source or API changes.

## 0.3.2

- Added `Manifold::slice_to_cross_section_with_fill_rule(height, fill_rule)`.
  The existing `slice_to_cross_section` keeps defaulting to `FillRule::Positive`;
  the new method lets you pick `EvenOdd` / `NonZero` / `Negative` for slices
  whose contour orientation you do not control. Additive; no changes required.

## 0.3.1

- Added the offline / bring-your-own-manifold build hatch
  (`MANIFOLD_CSG_LIB_DIR`) and bumped the upstream pin to manifold3d v3.5.1,
  plus `Manifold::*_with_context` constructors. Additive; no changes required.

## 0.3.0

Full guide: <https://github.com/zmerlynn/manifold-csg/blob/main/MIGRATION_0.2.0_TO_0.3.0.md>

- **Behavioral (silent):** the default fill rule flipped from `EvenOdd` to
  `Positive` in `CrossSection::from_polygons` and `Manifold::slice_to_cross_section`.
  Code compiles unchanged but the filled region can differ for self-intersecting
  or non-positively-oriented contours. To restore the old behavior, pass
  `FillRule::EvenOdd` to the `*_with_fill_rule` variant. (In 0.3.0 and 0.3.1,
  `slice_to_cross_section` had no such override; 0.3.2 adds
  `slice_to_cross_section_with_fill_rule`.)
- **API (compile-breaking):** `CrossSection::from_simple_polygon` dropped its
  `fill_rule` parameter (use `from_simple_polygon_with_fill_rule` for a
  non-default rule); status/mesh accessors moved to `Result`-returning forms;
  and `CsgError::EmptyMesh` was removed. These surface as compile errors.

## Earlier

- 0.1.8 -> 0.2.0: <https://github.com/zmerlynn/manifold-csg/blob/main/MIGRATION_0.1.8_TO_0.2.0.md>
- from 0.0.6: <https://github.com/zmerlynn/manifold-csg/blob/main/MIGRATION_FROM_0.0.6.md>
