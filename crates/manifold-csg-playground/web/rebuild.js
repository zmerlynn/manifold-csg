// Pure-data interface from the wasm module to a three.js BufferGeometry.
// Extracted from main.js so it can be unit-tested under Node without a
// browser/WebGL context.
//
// Both `THREE` and `wasm` are passed in by the caller — the module has no
// import-time dependencies of its own.

export const KIND_CUBE = 0;
export const KIND_SPHERE = 1;
export const KIND_CYLINDER = 2;
export const KIND_MENGER = 3;

export const OP_UNION = 0;
export const OP_DIFFERENCE = 1;
export const OP_INTERSECTION = 2;

export const DEFAULT_PARAMS = {
    [KIND_CUBE]:     [1.0, 1.0, 1.0, 0.0],
    [KIND_SPHERE]:   [0.7, 32.0, 0.0, 0.0],
    [KIND_CYLINDER]: [1.0, 0.5, 32.0, 0.0],
    // Menger sponge: p0 = recursion depth (clamped 0..=4 by the wasm side).
    // Level 2 = ~2k tris (instant); level 3 = ~30k (still snappy);
    // level 4 = ~400k (wasm-uu single-threaded gets sluggish). Default to 2.
    [KIND_MENGER]:   [2.0, 0.0, 0.0, 0.0],
};

// Copy a THREE.Matrix4 (column-major, length 16) into the wasm scratch
// buffer as a 12-element column-major 4x3 affine (manifold's convention:
// the implicit fourth row is [0,0,0,1]).
export function pushTransform(wasm, scratchPtr, slot, matrix4) {
    const e = matrix4.elements;
    const view = new Float64Array(wasm.memory.buffer, scratchPtr, 12);
    view[0]  = e[0];  view[1]  = e[1];  view[2]  = e[2];   // col0 (X basis)
    view[3]  = e[4];  view[4]  = e[5];  view[5]  = e[6];   // col1 (Y basis)
    view[6]  = e[8];  view[7]  = e[9];  view[8]  = e[10];  // col2 (Z basis)
    view[9]  = e[12]; view[10] = e[13]; view[11] = e[14];  // col3 (translation)
    wasm.set_transform(slot, scratchPtr);
}

export function setPrimitive(wasm, slot, kind, params = DEFAULT_PARAMS[kind]) {
    const [p0, p1, p2, p3] = params;
    wasm.set_primitive(slot, kind, p0, p1, p2, p3);
}

// Run the boolean and replace `mesh.geometry` with a fresh BufferGeometry
// holding the result. Returns the triangle count.
//
// We replace the BufferGeometry wholesale rather than mutating in place:
// `computeVertexNormals()` reuses an existing 'normal' attribute even when
// the new position attribute has a different vertex count, which then
// triggers WebGL "vertex buffer not big enough" once the rebuild grows
// vertex count.
export function rebuildIntoMesh(THREE, wasm, mesh) {
    const triCount = wasm.rebuild();

    const posPtr = wasm.positions_ptr();
    const posLen = wasm.positions_len();
    const idxPtr = wasm.indices_ptr();
    const idxLen = wasm.indices_len();

    // Slice copies — the wasm Vecs are reallocated on the next rebuild.
    const positions = new Float32Array(wasm.memory.buffer, posPtr, posLen).slice();
    const indices = new Uint32Array(wasm.memory.buffer, idxPtr, idxLen).slice();

    const geom = new THREE.BufferGeometry();
    geom.setAttribute('position', new THREE.BufferAttribute(positions, 3));
    geom.setIndex(new THREE.BufferAttribute(indices, 1));
    if (positions.length > 0) {
        geom.computeVertexNormals();
        geom.computeBoundingSphere();
    }
    mesh.geometry.dispose();
    mesh.geometry = geom;

    return triCount;
}
