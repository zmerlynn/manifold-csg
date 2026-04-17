//! # DEPRECATED
//!
//! This crate is deprecated. The `manifold3d` crate (0.1+) uses closures
//! for the `warp` and `set_properties` callbacks instead of proc macros,
//! so there is nothing left for this crate to provide.
//!
//! **Migration:** see the
//! [migration guide](https://github.com/zmerlynn/manifold-csg/blob/main/MIGRATION_FROM_0.0.6.md#callbacks-warp-and-set_properties).
//!
//! Using either attribute macro (`#[manifold_warp]` or
//! `#[manifold_manage_vertex_properties]`) from this crate will produce
//! a compile error pointing you at the migration guide.

use proc_macro::TokenStream;

/// DEPRECATED — use a closure with `Manifold::warp` instead.
///
/// See <https://github.com/zmerlynn/manifold-csg/blob/main/MIGRATION_FROM_0.0.6.md#warp>.
#[proc_macro_attribute]
pub fn manifold_warp(_attr: TokenStream, _input: TokenStream) -> TokenStream {
    r#"compile_error!("the #[manifold_warp] attribute is deprecated. The manifold3d crate (0.1+) uses closures for the warp callback instead of proc macros. Replace `#[manifold_warp] struct MyWarp; impl WarpVertex for MyWarp { ... }` with `manifold.warp(|x, y, z| [...])`. See https://github.com/zmerlynn/manifold-csg/blob/main/MIGRATION_FROM_0.0.6.md#warp");"#
        .parse()
        .unwrap()
}

/// DEPRECATED — use a closure with `Manifold::set_properties` instead.
///
/// See <https://github.com/zmerlynn/manifold-csg/blob/main/MIGRATION_FROM_0.0.6.md#set-properties>.
#[proc_macro_attribute]
pub fn manifold_manage_vertex_properties(_attr: TokenStream, _input: TokenStream) -> TokenStream {
    r#"compile_error!("the #[manifold_manage_vertex_properties] attribute is deprecated. The manifold3d crate (0.1+) uses closures for set_properties callbacks instead of proc macros. Replace `#[manifold_manage_vertex_properties] struct MyReplacer; impl ReplaceVertexProperties for MyReplacer { ... }` with `manifold.set_properties(num_prop, |new, pos, old| { ... })`. See https://github.com/zmerlynn/manifold-csg/blob/main/MIGRATION_FROM_0.0.6.md#set-properties");"#
        .parse()
        .unwrap()
}
