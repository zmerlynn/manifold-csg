# Changelog

## manifold-csg 0.1.0 (unreleased)

Initial release of safe Rust bindings to manifold3d v3.4.1.

### Highlights

- **Manifold** (3D solid): primitives (cube, cylinder, sphere, tetrahedron),
  boolean operations (union, difference, intersection with operator overloads),
  transforms, plane ops, convex hulls, Minkowski sum/difference, mesh I/O (f64
  and f32), OBJ I/O, SDF level-set construction, warp/set_properties callbacks,
  decompose, refine, smooth, and comprehensive query methods
- **CrossSection** (2D region): constructors, booleans with operator overloads,
  offset, hull, transforms (including 2D affine), warp, decompose, simplify,
  extrusion to 3D
- **BoundingBox** (3D AABB): spatial queries (contains, overlaps), union,
  transforms — 16 methods
- **Rect** (2D AABB): spatial queries, union, transforms — 16 methods
- **MeshGL64 / MeshGL**: f64 and f32 mesh data with Clone support
- **FillRule** enum for polygon fill rules
- **Triangulation**: constrained Delaunay triangulation of 2D polygons
- **Safety**: `catch_unwind` in all FFI callbacks, explicit `Send` + `!Sync`,
  `Drop` on all handle types
- **180 integration tests** covering all public methods, binding-specific
  concerns (memory safety, Send across threads, Drop cycles, FFI data
  integrity, ownership semantics), and edge cases
- Optional `nalgebra` feature for geometric type conversions

### manifold-csg-sys 3.4.100

Raw FFI bindings to the manifold3d v3.4.1 C API (221 functions).
Hand-written bindings (no bindgen dependency). Builds manifold3d from source
via cmake. Optional `parallel` feature for TBB threading.
