// Geometry-rebuild flow tests — verify the JS glue between wasm and three.js
// (rebuildIntoMesh) keeps the BufferGeometry in a state WebGL would accept,
// across sequences of rebuilds whose vertex count fluctuates.
//
// This test would have caught the original bug where `computeVertexNormals()`
// reused a too-small 'normal' attribute when vertex count grew between
// rebuilds, producing the WebGL "vertex buffer not big enough" error.

import test from 'node:test';
import assert from 'node:assert/strict';
import * as THREE from 'three';

import { loadWasm } from './load_wasm.mjs';
import {
    KIND_CUBE, OP_UNION, OP_INTERSECTION,
    pushTransform, setPrimitive, rebuildIntoMesh,
} from '../web/rebuild.js';

// The invariant WebGL would check at draw time: every attribute's count
// (= array.length / itemSize) must be ≥ max value in the index buffer + 1.
// Anything less and gl.drawElements rejects with "vertex buffer not big
// enough for the draw call".
function assertGeometryWebGLConsistent(geom, label) {
    const index = geom.index;
    if (!index || index.count === 0) {
        // Empty geometry is OK — three.js just skips the draw.
        return;
    }
    let maxIdx = -1;
    for (let i = 0; i < index.count; i++) {
        const v = index.getX(i);
        if (v > maxIdx) maxIdx = v;
    }
    const minVerts = maxIdx + 1;
    for (const [name, attr] of Object.entries(geom.attributes)) {
        assert.ok(
            attr.count >= minVerts,
            `${label}: attribute '${name}' has count ${attr.count}, ` +
            `but index buffer references vertex ${maxIdx} (need >= ${minVerts}). ` +
            `WebGL would reject this draw.`,
        );
    }
}

function makeMesh() {
    return new THREE.Mesh(new THREE.BufferGeometry());
}

function makeSetup(wasm) {
    const scratchPtr = wasm.alloc(96);
    wasm.set_op(OP_UNION);
    setPrimitive(wasm, 0, KIND_CUBE);
    setPrimitive(wasm, 1, KIND_CUBE);
    return { scratchPtr };
}

function translate(x) {
    const m = new THREE.Matrix4();
    m.makeTranslation(x, 0, 0);
    return m;
}

// ── Tests ──────────────────────────────────────────────────────────────

test('rebuildIntoMesh on default scene populates geometry consistently', async () => {
    const wasm = await loadWasm();
    const { scratchPtr } = makeSetup(wasm);
    pushTransform(wasm, scratchPtr, 0, new THREE.Matrix4());
    pushTransform(wasm, scratchPtr, 1, translate(0.7));
    const mesh = makeMesh();
    const tri = rebuildIntoMesh(THREE, wasm, mesh);
    assert.ok(tri > 0, `default union should be non-empty, got ${tri}`);
    assert.ok(mesh.geometry.attributes.position, 'position attribute set');
    assert.ok(mesh.geometry.attributes.normal, 'normal attribute set');
    assert.ok(mesh.geometry.index, 'index set');
    assertGeometryWebGLConsistent(mesh.geometry, 'default-scene');
});

test('rebuildIntoMesh stays WebGL-consistent across vertex-count fluctuations', async () => {
    // Regression test for the computeVertexNormals reuse bug: when vertex
    // count grows between rebuilds, the stale 'normal' attribute caused
    // WebGL "vertex buffer not big enough". We deliberately mix primitives,
    // ops, and translations to guarantee vertex count varies; then assert
    // every frame's geometry would be accepted by WebGL.
    const wasm = await loadWasm();
    const KIND_SPHERE = 1;
    const KIND_CYLINDER = 2;
    const OP_DIFFERENCE = 1;
    const { scratchPtr } = makeSetup(wasm);
    const mesh = makeMesh();

    const frames = [
        { aKind: KIND_CUBE,     bKind: KIND_CUBE,     bx: 0.7, op: OP_UNION },
        { aKind: KIND_SPHERE,   bKind: KIND_CUBE,     bx: 0.5, op: OP_UNION },
        { aKind: KIND_CYLINDER, bKind: KIND_SPHERE,   bx: 0.3, op: OP_DIFFERENCE },
        { aKind: KIND_CUBE,     bKind: KIND_SPHERE,   bx: 0.6, op: OP_INTERSECTION },
        { aKind: KIND_SPHERE,   bKind: KIND_CYLINDER, bx: 0.4, op: OP_UNION },
        { aKind: KIND_CYLINDER, bKind: KIND_CYLINDER, bx: 0.2, op: OP_DIFFERENCE },
    ];

    pushTransform(wasm, scratchPtr, 0, new THREE.Matrix4());
    const seenVertCounts = new Set();
    for (const f of frames) {
        setPrimitive(wasm, 0, f.aKind);
        setPrimitive(wasm, 1, f.bKind);
        wasm.set_op(f.op);
        pushTransform(wasm, scratchPtr, 1, translate(f.bx));
        rebuildIntoMesh(THREE, wasm, mesh);
        const label = `aKind=${f.aKind} bKind=${f.bKind} op=${f.op} bx=${f.bx}`;
        assertGeometryWebGLConsistent(mesh.geometry, label);
        seenVertCounts.add(mesh.geometry.attributes.position.count);
    }
    assert.ok(seenVertCounts.size >= 3,
        `sweep must hit at least 3 distinct vertex counts to exercise the bug; got ${seenVertCounts.size}: ${[...seenVertCounts].sort((a,b)=>a-b)}`);
});

test('rebuildIntoMesh handles empty result without leaving stale attributes', async () => {
    const wasm = await loadWasm();
    const { scratchPtr } = makeSetup(wasm);
    pushTransform(wasm, scratchPtr, 0, new THREE.Matrix4());
    pushTransform(wasm, scratchPtr, 1, translate(0.7));
    const mesh = makeMesh();

    // Populate with a non-empty result first so an old normal attribute exists.
    rebuildIntoMesh(THREE, wasm, mesh);
    assert.ok(mesh.geometry.attributes.position.count > 0);

    // Now move B far away and ask for the intersection — empty.
    pushTransform(wasm, scratchPtr, 1, translate(10.0));
    wasm.set_op(OP_INTERSECTION);
    const tri = rebuildIntoMesh(THREE, wasm, mesh);
    assert.equal(tri, 0, 'disjoint intersection should be empty');
    // After the empty rebuild, the geometry should still be drawable
    // (i.e., not reference indices into a stale larger position buffer).
    assertGeometryWebGLConsistent(mesh.geometry, 'empty-result');
    assert.equal(mesh.geometry.attributes.position.count, 0,
        'empty result should produce zero-vertex position attribute');
});

test('previous geometry is disposed when replaced', async () => {
    const wasm = await loadWasm();
    const { scratchPtr } = makeSetup(wasm);
    pushTransform(wasm, scratchPtr, 0, new THREE.Matrix4());
    pushTransform(wasm, scratchPtr, 1, translate(0.7));
    const mesh = makeMesh();

    rebuildIntoMesh(THREE, wasm, mesh);
    const firstGeom = mesh.geometry;
    let disposed = false;
    firstGeom.addEventListener('dispose', () => { disposed = true; });

    rebuildIntoMesh(THREE, wasm, mesh);
    assert.ok(disposed, 'previous BufferGeometry should be disposed on rebuild');
    assert.notEqual(mesh.geometry, firstGeom, 'rebuild should swap to a fresh geometry');
});

test('pushTransform writes column-major 4x3 layout into wasm scratch', async () => {
    const wasm = await loadWasm();
    const scratchPtr = wasm.alloc(96);
    const m = new THREE.Matrix4();
    // 90° about Y, then translate (5, 6, 7).
    m.makeRotationY(Math.PI / 2);
    m.setPosition(5, 6, 7);

    pushTransform(wasm, scratchPtr, 0, m);

    const view = new Float64Array(wasm.memory.buffer, scratchPtr, 12);
    // Column 0 = rotated X basis = (cos90, 0, -sin90) = (0, 0, -1)
    assert.ok(Math.abs(view[0] - 0) < 1e-9);
    assert.ok(Math.abs(view[1] - 0) < 1e-9);
    assert.ok(Math.abs(view[2] - -1) < 1e-9);
    // Column 1 = Y basis = (0, 1, 0)
    assert.ok(Math.abs(view[3] - 0) < 1e-9);
    assert.ok(Math.abs(view[4] - 1) < 1e-9);
    assert.ok(Math.abs(view[5] - 0) < 1e-9);
    // Column 2 = rotated Z basis = (sin90, 0, cos90) = (1, 0, 0)
    assert.ok(Math.abs(view[6] - 1) < 1e-9);
    assert.ok(Math.abs(view[7] - 0) < 1e-9);
    assert.ok(Math.abs(view[8] - 0) < 1e-9);
    // Column 3 = translation
    assert.equal(view[9], 5);
    assert.equal(view[10], 6);
    assert.equal(view[11], 7);
});
