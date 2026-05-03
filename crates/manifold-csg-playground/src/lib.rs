//! Boolean playground — minimal C ABI bridging `manifold-csg` to a browser
//! frontend (three.js).
//!
//! The state is two primitive "slots" plus a boolean op selector. Each slot
//! has a kind (cube/sphere/cylinder/menger), shape parameters, and a 4x3 affine
//! transform pushed in from the JS gizmo. `rebuild()` runs the boolean and
//! caches the resulting f32 vertex positions + u32 triangle indices in
//! globally-owned `Vec`s; the frontend reads them via raw pointer + length
//! getters and copies into a `THREE.BufferGeometry`.
//!
//! Designed for `wasm32-unknown-unknown` (no wasm-bindgen — see PR #34's
//! `docs/plans/wasm-unknown-unknown.md` for why), but the same C ABI works
//! on host so `cargo test` / `cargo check` on the workspace stay green.

use std::sync::{Mutex, OnceLock};

use manifold_csg::Manifold;

#[derive(Clone)]
struct Slot {
    kind: i32,
    params: [f64; 4],
    transform: [f64; 12],
}

const IDENTITY_4X3: [f64; 12] = [
    1.0, 0.0, 0.0, // col0 (X basis)
    0.0, 1.0, 0.0, // col1 (Y basis)
    0.0, 0.0, 1.0, // col2 (Z basis)
    0.0, 0.0, 0.0, // col3 (translation)
];

impl Slot {
    fn build(&self) -> Manifold {
        let prim = match self.kind {
            0 => Manifold::cube(self.params[0], self.params[1], self.params[2], true),
            1 => {
                let segs = (self.params[1] as i32).max(8);
                Manifold::sphere(self.params[0], segs)
            }
            2 => {
                let segs = (self.params[2] as i32).max(8);
                Manifold::cylinder(self.params[0], self.params[1], self.params[1], segs, true)
            }
            3 => {
                // Menger sponge — recursion depth in p0. Clamp to [0, 4]:
                // upstream warns level 4 already produces ~400k triangles,
                // and the demo wants to stay interactive.
                let level = (self.params[0] as i32).clamp(0, 4) as u32;
                manifold_csg::samples::menger_sponge(level)
            }
            _ => Manifold::empty(),
        };
        prim.transform(&self.transform)
    }
}

struct State {
    a: Slot,
    b: Slot,
    op: i32,
    last_positions: Vec<f32>,
    last_indices: Vec<u32>,
}

fn state() -> &'static Mutex<State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE.get_or_init(|| {
        // Default scene: Menger sponge ∩ sphere, halfway overlapping. Picks
        // a fast-evaluating boolean that obviously isn't either input shape,
        // so visitors landing on the demo immediately see "this is doing CSG"
        // rather than "this is just a cube".
        let mut b_xform = IDENTITY_4X3;
        b_xform[9] = 0.5; // sphere center on the menger's +X face

        Mutex::new(State {
            a: Slot {
                kind: 3,                      // menger sponge
                params: [2.0, 0.0, 0.0, 0.0], // recursion depth 2
                transform: IDENTITY_4X3,
            },
            b: Slot {
                kind: 1,                       // sphere
                params: [0.7, 32.0, 0.0, 0.0], // radius, segments
                transform: b_xform,
            },
            op: 2, // intersection
            last_positions: Vec::new(),
            last_indices: Vec::new(),
        })
    })
}

// ── JS-facing memory helpers ────────────────────────────────────────────

/// Allocate `n` zero-initialised bytes in wasm linear memory and return a
/// raw pointer. Caller (JS) is expected to release with [`dealloc`] when
/// done.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(n: usize) -> *mut u8 {
    let mut v = vec![0u8; n];
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

/// Free a buffer previously returned by [`alloc`].
///
/// # Safety
///
/// `ptr` must have been returned by `alloc(n)` and not freed since. `n`
/// must equal the original allocation size.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, n: usize) {
    if ptr.is_null() || n == 0 {
        return;
    }
    // SAFETY: per function contract, ptr came from alloc(n) (a Vec<u8> with
    // capacity == length == n) and has not been freed; reconstruction with
    // matching len/cap restores the original Vec for drop.
    let v = unsafe { Vec::from_raw_parts(ptr, n, n) };
    drop(v);
}

// ── Slot configuration ──────────────────────────────────────────────────

fn slot_mut(s: &mut State, slot: i32) -> &mut Slot {
    match slot {
        0 => &mut s.a,
        1 => &mut s.b,
        n => panic!("invalid slot {n}: expected 0 (A) or 1 (B)"),
    }
}

/// Set primitive kind + parameters for a slot. `slot` is 0 (A) or 1 (B).
/// `kind` is 0=cube, 1=sphere, 2=cylinder, 3=menger. The four `p*`
/// parameters are shape-specific:
///
/// - cube: `(x, y, z, _)`
/// - sphere: `(radius, segments, _, _)`
/// - cylinder: `(height, radius, segments, _)`
/// - menger: `(level, _, _, _)` — recursion depth, clamped to [0, 4]
///
/// Panics if `slot` is not 0 or 1. (Panics in this crate abort the wasm
/// instance; the JS frontend is expected to pass valid values.)
#[unsafe(no_mangle)]
pub extern "C" fn set_primitive(slot: i32, kind: i32, p0: f64, p1: f64, p2: f64, p3: f64) {
    let mut s = state().lock().expect("playground state lock poisoned");
    let target = slot_mut(&mut s, slot);
    target.kind = kind;
    target.params = [p0, p1, p2, p3];
}

/// Update the 4x3 column-major affine transform for a slot.
///
/// Panics if `slot` is not 0 or 1.
///
/// # Safety
///
/// `m_ptr` must point to 12 contiguous, well-aligned `f64`s.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn set_transform(slot: i32, m_ptr: *const f64) {
    // SAFETY: per function contract, m_ptr is valid for 12 f64 reads.
    let m = unsafe { std::slice::from_raw_parts(m_ptr, 12) };
    let mut s = state().lock().expect("playground state lock poisoned");
    let target = slot_mut(&mut s, slot);
    target.transform.copy_from_slice(m);
}

/// Set the boolean operation: 0=union, 1=difference (A − B), 2=intersection.
/// Panics on any other value.
#[unsafe(no_mangle)]
pub extern "C" fn set_op(op: i32) {
    if !(0..=2).contains(&op) {
        panic!("invalid op {op}: expected 0 (union), 1 (difference), or 2 (intersection)");
    }
    state().lock().expect("playground state lock poisoned").op = op;
}

// ── Recompute + result accessors ───────────────────────────────────────

/// Recompute the boolean. Returns the resulting triangle count, or 0 if
/// the result is empty. The returned positions and indices are valid until
/// the next call to `rebuild`.
#[unsafe(no_mangle)]
pub extern "C" fn rebuild() -> i32 {
    let mut s = state().lock().expect("playground state lock poisoned");
    let a = s.a.build();
    let b = s.b.build();
    // `op` is validated at set_op time, so any value here is one of {0,1,2}.
    let result = match s.op {
        0 => &a + &b,
        1 => &a - &b,
        2 => &a ^ &b,
        n => unreachable!("op {n} should have been rejected by set_op"),
    };
    let (verts, n_props, indices) = result.to_mesh_f32();

    // Strip down to xyz only if the manifold carried extra properties
    // (it shouldn't for our use case, but defensive).
    let positions = if n_props == 3 || verts.is_empty() {
        verts
    } else {
        let n_verts = verts.len() / n_props;
        let mut p = Vec::with_capacity(n_verts * 3);
        for i in 0..n_verts {
            let base = i * n_props;
            p.push(verts[base]);
            p.push(verts[base + 1]);
            p.push(verts[base + 2]);
        }
        p
    };

    let n_tris = indices.len() / 3;
    s.last_positions = positions;
    s.last_indices = indices;
    n_tris.min(i32::MAX as usize) as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn positions_ptr() -> *const f32 {
    state()
        .lock()
        .expect("playground state lock poisoned")
        .last_positions
        .as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn positions_len() -> usize {
    state()
        .lock()
        .expect("playground state lock poisoned")
        .last_positions
        .len()
}

#[unsafe(no_mangle)]
pub extern "C" fn indices_ptr() -> *const u32 {
    state()
        .lock()
        .expect("playground state lock poisoned")
        .last_indices
        .as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn indices_len() -> usize {
    state()
        .lock()
        .expect("playground state lock poisoned")
        .last_indices
        .len()
}
