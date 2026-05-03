//! Sample CSG constructions ported from manifold3d's `samples/` C++ sources.
//!
//! These mirror the upstream samples (faithful translations of the algorithms,
//! not 1:1 line-for-line) but live entirely in our safe Rust layer — the shim
//! deliberately doesn't compile manifold's `samples/` directory, so the
//! upstream functions aren't available as C symbols.

use crate::Manifold;

/// Menger sponge — the classic cubic fractal.
///
/// `n` is the recursion depth. **Warning:** triangle count grows exponentially
/// — `n = 4` produces ~400k triangles. `n = 2` is a good interactive default.
///
/// Centered unit cube with cross-shaped tunnels carved through each face,
/// recursively. Translated from manifold3d's
/// [`MengerSponge`](https://github.com/elalish/manifold/blob/master/samples/src/menger_sponge.cpp)
/// sample.
#[must_use]
pub fn menger_sponge(n: u32) -> Manifold {
    let result = Manifold::cube(1.0, 1.0, 1.0, true);
    // Level 0 is just the cube (no tunnels). Bail before calling `fractal`
    // — its `depth == max_depth` exit condition would never trigger when
    // started with depth=1, max_depth=0, infinite-recursing. (Same latent
    // bug in the upstream C++ sample we ported from.)
    if n == 0 {
        return result;
    }

    let mut holes: Vec<Manifold> = Vec::new();
    fractal(&mut holes, &result, 1.0, [0.0, 0.0], 1, n);

    let hole = Manifold::batch_union(&holes);
    let r1 = &result - &hole;
    // Rotate the union of tunnels 90° about X to get tunnels along Y, then
    // 90° about Z to get tunnels along X.
    let hole_y = hole.rotate(90.0, 0.0, 0.0);
    let r2 = &r1 - &hole_y;
    let hole_x = hole_y.rotate(0.0, 0.0, 90.0);
    &r2 - &hole_x
}

/// Recursive helper: build the set of square tunnels (along Z) that get
/// subtracted from the cube to form one face's worth of the Menger pattern.
/// Mirrors upstream's nested `Fractal` lambda.
fn fractal(
    holes: &mut Vec<Manifold>,
    hole: &Manifold,
    w: f64,
    position: [f64; 2],
    depth: u32,
    max_depth: u32,
) {
    let w = w / 3.0;
    holes.push(
        hole.scale(w, w, 1.0)
            .translate(position[0], position[1], 0.0),
    );
    if depth == max_depth {
        return;
    }
    let offsets = [
        [-w, -w],
        [-w, 0.0],
        [-w, w],
        [0.0, w],
        [w, w],
        [w, 0.0],
        [w, -w],
        [0.0, -w],
    ];
    for off in offsets {
        fractal(
            holes,
            hole,
            w,
            [position[0] + off[0], position[1] + off[1]],
            depth + 1,
            max_depth,
        );
    }
}
