//! Safe Rust bindings to [manifold3d](https://github.com/elalish/manifold) —
//! a geometry kernel for constructive solid geometry (CSG).
//!
//! This crate provides safe, ergonomic wrappers around the manifold3d C API.
//! For details on the underlying algorithms and behavior, see the
//! [upstream documentation](https://elalish.github.io/manifold/docs/html/).
//!
//! - [`Manifold`] — 3D solid with boolean operations (union, difference, intersection)
//! - [`CrossSection`] — 2D region with offset, boolean, and hull operations
//! - [`MeshGL64`] / [`MeshGL`] — mesh data transfer (f64 and f32 precision)
//! - [`triangulate_polygons`] — constrained Delaunay triangulation of 2D polygons
//!
//! # Key features
//!
//! - **f64 precision**: Uses MeshGL64 to avoid f32 precision loss
//! - **`Send` + `Sync` safe**: All types can be moved across threads and shared for concurrent reads
//! - **Memory safe**: All C handles are freed automatically via `Drop`
//! - **Browser-capable**: Builds for `wasm32-unknown-emscripten` and
//!   `wasm32-unknown-unknown` (the wasm-bindgen-compatible target) for
//!   in-browser geometry. See the README's "Browser / WebAssembly" sections.
//!
//! # Upgrading
//!
//! See [`CHANGELOG.md`](https://github.com/zmerlynn/manifold-csg/blob/main/crates/manifold-csg/CHANGELOG.md)
//! (also shipped inside the crate) for per-version release notes. It flags
//! *behavioral* changes that compile cleanly but change output - notably the
//! `EvenOdd` -> `Positive` default fill-rule flip in 0.3.0 - alongside the
//! full migration guides for compile-breaking changes.

pub mod bounding_box;
pub mod cross_section;
pub mod execution;
pub mod manifold;
pub mod mesh;
pub mod ray;
pub mod rect;
pub mod samples;
pub mod triangulation;
pub mod types;

pub use bounding_box::BoundingBox;
pub use cross_section::{CrossSection, FillRule, JoinType, Rect2};
pub use execution::ExecutionContext;
pub use manifold::Manifold;
pub use manifold::{
    get_circular_segments, get_relative_precision, manifold_version, reserve_ids,
    reset_to_circular_defaults, set_circular_segments, set_min_circular_angle,
    set_min_circular_edge_length, set_relative_precision,
};
pub use mesh::{MeshGL, MeshGL64, MeshGL64Options, MeshGLOptions};
pub use ray::RayHit;
pub use rect::Rect;
pub use triangulation::triangulate_polygons;
pub use types::{CsgError, OpType};

/// Takes a handle the C API just returned.
///
/// Every `manifold_*` function that produces a handle heap-allocates it with
/// C++ `new`, which throws rather than returning null, so a null here is an
/// upstream bug. Panicking keeps it from becoming a null dereference later.
/// The caller owns the handle and frees it with the matching
/// `manifold_delete_*`.
pub(crate) fn owned<T>(ptr: *mut T) -> *mut T {
    assert!(!ptr.is_null(), "manifold C API returned a null handle");
    ptr
}

/// Copies `len` elements out of storage the C API lends, such as a mesh's
/// vertex array.
///
/// # Safety
///
/// When `len > 0`, `data` must point at `len` initialized elements that stay
/// valid for the duration of this call. When `len == 0`, `data` may be null
/// (an empty C++ vector's `data()` can be).
pub(crate) unsafe fn copy_out<T: Copy>(data: *const T, len: usize) -> Vec<T> {
    if len == 0 {
        return Vec::new();
    }
    // SAFETY: the caller guarantees data addresses len initialized elements.
    unsafe { std::slice::from_raw_parts(data, len) }.to_vec()
}
