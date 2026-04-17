//! Raw FFI bindings to the manifold3d C API.
//!
//! This crate is a thin re-export of [`manifold-csg-sys`](https://crates.io/crates/manifold-csg-sys).
//! The two crates provide the same FFI surface under different names — use
//! whichever one you prefer.
//!
//! **If you are migrating from `manifold3d-sys` 0.0.x (the original crate by
//! @NickUfer, now transferred to this project), see the
//! [migration guide](https://github.com/zmerlynn/manifold-csg/blob/main/MIGRATION_FROM_0.0.6.md).**
//! The API has changed significantly between 0.0.x and 3.4+.

pub use manifold_csg_sys::*;
