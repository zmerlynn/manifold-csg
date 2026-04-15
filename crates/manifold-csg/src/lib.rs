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

pub mod bounding_box;
pub mod cross_section;
pub mod manifold;
pub mod mesh;
pub mod ray;
pub mod rect;
pub mod triangulation;
pub mod types;

pub use bounding_box::BoundingBox;
pub use cross_section::{CrossSection, FillRule, JoinType, Rect2};
pub use manifold::Manifold;
pub use manifold::{
    get_circular_segments, reserve_ids, reset_to_circular_defaults, set_circular_segments,
    set_min_circular_angle, set_min_circular_edge_length,
};
pub use manifold_csg_sys::ManifoldOpType as OpType;
pub use mesh::{MeshGL, MeshGL64};
pub use ray::RayHit;
pub use rect::Rect;
pub use triangulation::triangulate_polygons;
pub use types::CsgError;
