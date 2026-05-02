//! Forward link arguments emitted by `manifold-csg-sys` to downstream link steps.
//!
//! The sys crate exposes target-specific link flags (e.g. `-fwasm-exceptions`,
//! `-sSTACK_SIZE` for emscripten) via `cargo:link_args=...`, which Cargo
//! translates into the `DEP_MANIFOLD_LINK_ARGS` env var visible to the build
//! scripts of crates that depend on it. Cargo does NOT auto-propagate
//! `rustc-link-arg` from a sys crate to downstream link invocations, so we
//! re-emit them here so they reach binaries / tests / cdylibs that depend on
//! `manifold-csg`.
//!
//! End-user crates that produce their own bin/cdylib still need a similar
//! one-liner in their own build.rs (or a `.cargo/config.toml` entry) — see
//! the README's "Browser / WebAssembly" section.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=DEP_MANIFOLD_LINK_ARGS");

    if let Ok(args) = std::env::var("DEP_MANIFOLD_LINK_ARGS") {
        for arg in args.split_whitespace() {
            println!("cargo:rustc-link-arg={arg}");
        }
    }
}
