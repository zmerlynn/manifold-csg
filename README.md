# manifold-csg

[![crates.io](https://img.shields.io/crates/v/manifold-csg.svg)](https://crates.io/crates/manifold-csg)
[![docs.rs](https://docs.rs/manifold-csg/badge.svg)](https://docs.rs/manifold-csg)
[![CI](https://github.com/zmerlynn/manifold-csg/actions/workflows/ci.yml/badge.svg)](https://github.com/zmerlynn/manifold-csg/actions/workflows/ci.yml)

Safe Rust bindings to the [manifold3d](https://github.com/elalish/manifold)
geometry kernel for constructive solid geometry (CSG).

manifold3d is a fast, robust C++ library for boolean operations on 3D triangle
meshes. These bindings make its capabilities accessible from Rust with minimal
overhead and without requiring users to manage C pointers or memory. See the
[upstream documentation](https://elalish.github.io/manifold/docs/html/) for
details on the underlying algorithms and behavior.

## What's included

**`manifold-csg-sys`** provides raw FFI bindings to the manifold3d C API. If
you need direct C-level control, it's there.

**`manifold-csg`** wraps the most commonly needed operations in safe Rust:

- **3D solids** ([`Manifold`](crates/manifold-csg/src/manifold.rs)) — primitives
  (cube, sphere, cylinder, tetrahedron), boolean operations (union, difference,
  intersection), transforms, convex hull, decomposition, Minkowski sum/difference,
  mesh refinement, smoothing, SDF level sets, warp deformation, and OBJ I/O
- **2D regions** ([`CrossSection`](crates/manifold-csg/src/cross_section.rs)) —
  primitives (square, circle, polygons), boolean operations, Clipper2-based
  geometric offset, convex hull, transforms, warp, and simplification
- **Mesh data** ([`MeshGL64`](crates/manifold-csg/src/mesh.rs) /
  [`MeshGL`](crates/manifold-csg/src/mesh.rs)) — f64 and f32 mesh types for
  getting data in and out
- **Triangulation** ([`triangulate_polygons`](crates/manifold-csg/src/triangulation.rs))
  — constrained Delaunay triangulation of 2D polygons
- **2D-to-3D** — extrude (with optional twist and scale) and revolve cross-sections
  into solids; slice solids back to cross-sections

See [API_COVERAGE.md](API_COVERAGE.md) for a full table mapping every C API function
to its safe wrapper (or noting where one doesn't exist yet).

## Design choices

- **f64 by default.** Mesh I/O uses `MeshGL64` so you don't lose precision
  through f32 round-trips. f32 paths (`from_mesh_f32`, `to_mesh_f32`, `MeshGL`)
  are available when you need them.
- **`Send` + `Sync`.** All types can be moved across threads and shared for
  concurrent reads.
- **Automatic memory management.** All C handles are freed via `Drop`. No manual
  cleanup needed.
- **Operator overloads.** `&a + &b` (union), `&a - &b` (difference), `&a ^ &b`
  (intersection) work on both `Manifold` and `CrossSection`.
- **Callback-based APIs wrapped safely.** `warp`, `set_properties`, `from_sdf`,
  and OBJ I/O all accept closures with `catch_unwind` to prevent panics from
  unwinding through C stack frames.
- **C API parity.** Parameter order and names follow the C API so users
  transitioning from C/C++ find things where they expect.

## Quick start

```rust
use manifold_csg::{Manifold, CrossSection, JoinType};

// 3D: drill a cylindrical hole through a cube
let cube = Manifold::cube(20.0, 20.0, 20.0, true);
let hole = Manifold::cylinder(30.0, 5.0, 5.0, 32, false);
let result = &cube - &hole;
assert!(result.volume() < cube.volume());

// 2D -> 3D: offset a rectangle and extrude it
let section = CrossSection::square(10.0, 10.0, true);
let expanded = section.offset(2.0, JoinType::Round, 2.0, 16);
let solid = expanded.extrude(20.0);
```

See the [`examples/`](crates/manifold-csg/examples/) directory for more
complete, runnable examples.

## Crates

| Crate | Description |
|-------|-------------|
| [`manifold-csg`](crates/manifold-csg/) | Safe Rust wrapper (start here) |
| [`manifold-csg-sys`](crates/manifold-csg-sys/) | Raw FFI bindings to the full C API |

## Build requirements

- Rust 1.85+
- git, cmake, a C++ compiler
- First build clones manifold3d and compiles it from source; subsequent builds
  use the cached copy. Internet access is required for the initial clone.

```sh
cargo build           # builds both crates
cargo test --features nalgebra   # runs the test suite
```

Tested on Linux, macOS, and Windows.

## Feature flags

| Feature | Default | Description |
|---------|---------|-------------|
| `parallel` | yes | Enables TBB-based parallelism for boolean operations |
| `nalgebra` | no | Adds convenience methods that accept `nalgebra::Matrix3`, `Vector3`, `Point3` |

## Documentation

- **[API_COVERAGE.md](API_COVERAGE.md)** — maps every manifold3d C function to
  its safe wrapper, with source links
- **[docs.rs](https://docs.rs/manifold-csg)** — generated API docs
- **[examples/](crates/manifold-csg/examples/)** — runnable code examples
- **[Upstream docs](https://elalish.github.io/manifold/docs/html/)** — manifold3d
  C++ API documentation (helpful for understanding parameter semantics)

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
