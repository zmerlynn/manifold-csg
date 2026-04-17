//! Safe Rust bindings to [manifold3d](https://github.com/elalish/manifold) —
//! a geometry kernel for constructive solid geometry (CSG).
//!
//! This crate is a thin re-export of [`manifold-csg`](https://crates.io/crates/manifold-csg).
//! The two crates provide the same API under different names — use whichever
//! one you prefer.
//!
//! **If you are migrating from `manifold3d` 0.0.x (the original crate by
//! @NickUfer, now transferred to this project), see the
//! [migration guide](https://github.com/zmerlynn/manifold-csg/blob/main/MIGRATION_FROM_0.0.6.md).**
//! The API has changed significantly between 0.0.x and 0.1+.

pub use manifold_csg::*;
