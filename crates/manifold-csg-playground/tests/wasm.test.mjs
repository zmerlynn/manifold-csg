// Wasm C ABI tests.
//
// Drives the same exported functions the browser frontend calls
// (set_primitive / set_transform / set_op / rebuild + position/index
// accessors) and verifies internal consistency: triangle counts make sense,
// indices stay in-bounds of the position buffer, no NaNs, distinct ops
// produce distinct results.

import test from 'node:test';
import assert from 'node:assert/strict';
import { loadWasm } from './load_wasm.mjs';

const KIND_CUBE = 0;
const KIND_SPHERE = 1;
const KIND_CYLINDER = 2;

const OP_UNION = 0;
const OP_DIFFERENCE = 1;
const OP_INTERSECTION = 2;

const IDENTITY_4X3 = [1, 0, 0,  0, 1, 0,  0, 0, 1,  0, 0, 0];
const TRANSLATE_X = (x) => [1, 0, 0,  0, 1, 0,  0, 0, 1,  x, 0, 0];

function readPositions(wasm) {
    return new Float32Array(
        wasm.memory.buffer, wasm.positions_ptr(), wasm.positions_len(),
    ).slice();
}
function readIndices(wasm) {
    return new Uint32Array(
        wasm.memory.buffer, wasm.indices_ptr(), wasm.indices_len(),
    ).slice();
}

function pushTransform(wasm, scratchPtr, slot, m12) {
    const view = new Float64Array(wasm.memory.buffer, scratchPtr, 12);
    for (let i = 0; i < 12; i++) view[i] = m12[i];
    wasm.set_transform(slot, scratchPtr);
}

// Set up a fresh wasm instance with two unit cubes (B translated +0.7 X)
// and union op. Returns { wasm, scratchPtr } that match the browser's
// initial scene.
async function defaultScene() {
    const wasm = await loadWasm();
    const scratchPtr = wasm.alloc(96);
    wasm.set_op(OP_UNION);
    wasm.set_primitive(0, KIND_CUBE, 1.0, 1.0, 1.0, 0.0);
    wasm.set_primitive(1, KIND_CUBE, 1.0, 1.0, 1.0, 0.0);
    pushTransform(wasm, scratchPtr, 0, IDENTITY_4X3);
    pushTransform(wasm, scratchPtr, 1, TRANSLATE_X(0.7));
    return { wasm, scratchPtr };
}

// ── Property assertions ───────────────────────────────────────────────

function assertResultConsistent(wasm, triCount, label) {
    const positions = readPositions(wasm);
    const indices = readIndices(wasm);

    assert.equal(indices.length, triCount * 3,
        `${label}: indices.length should equal triCount*3`);
    assert.equal(positions.length % 3, 0,
        `${label}: positions.length should be a multiple of 3 (xyz)`);

    if (triCount === 0) {
        assert.equal(positions.length, 0, `${label}: empty result has no positions`);
        return;
    }

    const vertCount = positions.length / 3;
    let maxIdx = -1;
    for (const i of indices) if (i > maxIdx) maxIdx = i;
    assert.ok(maxIdx < vertCount,
        `${label}: max index ${maxIdx} must be < vertex count ${vertCount}`);

    for (const v of positions) {
        assert.ok(Number.isFinite(v), `${label}: position contains non-finite ${v}`);
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

test('alloc/dealloc returns non-null aligned pointer', async () => {
    const wasm = await loadWasm();
    const p = wasm.alloc(96);
    assert.notEqual(p, 0);
    assert.equal(p % 8, 0, 'alloc(96) result should be 8-byte aligned');
    wasm.dealloc(p, 96);
});

test('default scene: two unit cubes overlapping → union has > 0 triangles', async () => {
    const { wasm } = await defaultScene();
    const triCount = wasm.rebuild();
    assert.ok(triCount > 0, `expected non-empty union, got ${triCount}`);
    assertResultConsistent(wasm, triCount, 'default-union');
});

test('union vs intersection vs difference produce different triangle counts', async () => {
    const { wasm } = await defaultScene();
    wasm.set_op(OP_UNION);
    const u = wasm.rebuild();
    wasm.set_op(OP_INTERSECTION);
    const i = wasm.rebuild();
    wasm.set_op(OP_DIFFERENCE);
    const d = wasm.rebuild();
    assert.ok(u > 0 && i > 0 && d > 0, `all ops should be non-empty: u=${u} i=${i} d=${d}`);
    // A ∪ B is strictly larger than A ∩ B for two overlapping cubes.
    assert.ok(u > i, `union (${u}) should have more tris than intersection (${i})`);
});

test('disjoint operands: intersection is empty, union is non-empty', async () => {
    const { wasm, scratchPtr } = await defaultScene();
    pushTransform(wasm, scratchPtr, 1, TRANSLATE_X(10.0)); // far apart
    wasm.set_op(OP_INTERSECTION);
    assert.equal(wasm.rebuild(), 0, 'disjoint cubes have empty intersection');
    assertResultConsistent(wasm, 0, 'disjoint-intersection');
    wasm.set_op(OP_UNION);
    const u = wasm.rebuild();
    assert.ok(u > 0, 'disjoint cubes have non-empty union');
    assertResultConsistent(wasm, u, 'disjoint-union');
});

test('rebuild result is internally consistent across primitive switches', async () => {
    const { wasm } = await defaultScene();
    const cases = [
        { a: KIND_CUBE,     b: KIND_SPHERE,   ap: [1, 1, 1, 0],     bp: [0.7, 32, 0, 0] },
        { a: KIND_SPHERE,   b: KIND_CYLINDER, ap: [0.7, 32, 0, 0],  bp: [1, 0.5, 32, 0] },
        { a: KIND_CYLINDER, b: KIND_CUBE,     ap: [1, 0.5, 32, 0],  bp: [1, 1, 1, 0] },
    ];
    for (const { a, b, ap, bp } of cases) {
        wasm.set_primitive(0, a, ...ap);
        wasm.set_primitive(1, b, ...bp);
        wasm.set_op(OP_UNION);
        const triCount = wasm.rebuild();
        assertResultConsistent(wasm, triCount, `union(kind=${a},kind=${b})`);
    }
});

test('repeated rebuilds with growing/shrinking results stay consistent', async () => {
    // Reproduces the "dragging the gizmo" pattern that surfaced the
    // computeVertexNormals reuse bug in main.js — vertex count fluctuates
    // across rebuilds. The wasm side's job is to keep its own outputs
    // self-consistent; the JS test (rebuild.test.mjs) covers the JS bug.
    const { wasm, scratchPtr } = await defaultScene();
    for (const x of [0.1, 0.4, 0.7, 1.2, 0.3, 0.9, 0.05, 0.6]) {
        pushTransform(wasm, scratchPtr, 1, TRANSLATE_X(x));
        const triCount = wasm.rebuild();
        assertResultConsistent(wasm, triCount, `union with B at x=${x}`);
    }
});

test('result pointers are stable across reads but invalidated by next rebuild', async () => {
    const { wasm } = await defaultScene();
    wasm.rebuild();
    const p1 = wasm.positions_ptr();
    const p2 = wasm.positions_ptr();
    assert.equal(p1, p2, 'consecutive positions_ptr() reads return the same pointer');

    // After another rebuild with a different op (which produces different
    // result sizes), the pointer may legitimately change.
    wasm.set_op(OP_INTERSECTION);
    wasm.rebuild();
    // We don't assert p1 vs new — both reuse-and-different are legal — but
    // the new read must be internally consistent.
    assertResultConsistent(wasm, wasm.indices_len() / 3, 'after-op-switch');
});
