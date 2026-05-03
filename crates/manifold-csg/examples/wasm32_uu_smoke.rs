//! Browser-runnable smoke test for the wasm32-unknown-unknown build.
//!
//! Builds for wasm32-unknown-unknown via:
//!
//! ```sh
//! cargo build --example wasm32_uu_smoke \
//!     --target wasm32-unknown-unknown -p manifold-csg \
//!     --no-default-features --features unstable-wasm-uu
//! ```
//!
//! Then load the resulting `.wasm` from a Node/JS runner and call the
//! exported `smoke_run` function — see
//! `crates/manifold-csg/wasm32-uu-runner/run.mjs` for a working example.
//!
//! `smoke_run` returns the post-union triangle count (positive integer);
//! a zero or negative value means something is broken.
//!
//! This example only compiles for wasm32-unknown-unknown; on other targets
//! it's a stub `main` that prints a usage hint.

#![cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), no_main)]

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod wasm32_uu {
    use manifold_csg::Manifold;

    /// Returns the post-union triangle count for two unit cubes offset by
    /// (0.5, 0.5, 0.5). Exercises construction, transform, and the boolean
    /// kernel — enough to pull manifold's actual CSG path through linker
    /// symbol resolution.
    ///
    /// Exported via `#[unsafe(no_mangle)] pub extern "C"` so a JS runner can
    /// invoke it directly. Build with
    /// `RUSTFLAGS='-C link-arg=--export=smoke_run' cargo build ...` if the
    /// linker doesn't auto-export it (rustc on wasm32-unknown-unknown
    /// usually does, but this is explicit insurance).
    #[unsafe(no_mangle)]
    pub extern "C" fn smoke_run() -> i32 {
        let cube1 = Manifold::cube(1.0, 1.0, 1.0, true);
        let cube2 = cube1.translate(0.5, 0.5, 0.5);
        let merged = &cube1 + &cube2;
        // Cap at i32::MAX rather than returning -1 on overflow — the
        // runner treats negative as failure, but an overflowed positive
        // count would still be honestly reportable. For two unit cubes
        // the value is small (36 today), so this is a documentation
        // hint more than a real concern.
        merged.num_tri().min(i32::MAX as usize) as i32
    }
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
fn main() {
    eprintln!(
        "wasm32_uu_smoke is the wasm32-unknown-unknown smoke test.\n\
         Build it via:\n  \
         cargo build --example wasm32_uu_smoke --target wasm32-unknown-unknown \
         -p manifold-csg --no-default-features --features unstable-wasm-uu\n\
         Then run via the Node script at \
         crates/manifold-csg/wasm32-uu-runner/run.mjs."
    );
}
