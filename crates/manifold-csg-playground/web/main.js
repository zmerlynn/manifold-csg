// manifold-csg-playground — three.js frontend
//
// Loads the wasm32-unknown-unknown build of `manifold-csg-playground`, wires
// two TransformControls gizmos to push 4×3 affine matrices into wasm, and
// reads the boolean-result mesh back as `BufferGeometry` attributes from
// raw linear memory.
//
// Zero npm dependencies — three.js is pulled from a CDN via the import map
// in index.html. PR #34 (wasm32-unknown-unknown support) deliberately
// punts wasm-bindgen, so this uses the raw C-style ABI exported by lib.rs.
//
// The pure-data parts (matrix conversion, geometry rebuild) live in
// rebuild.js so they can be unit-tested under Node.

import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { TransformControls } from 'three/addons/controls/TransformControls.js';

import {
    KIND_CUBE, KIND_SPHERE, KIND_CYLINDER, KIND_MENGER,
    OP_INTERSECTION,
    pushTransform as pushTransformImpl,
    setPrimitive as setPrimitiveImpl,
    rebuildIntoMesh,
} from './rebuild.js';

// ── Status helper ──────────────────────────────────────────────────────

const statusEl = document.getElementById('status');
function setStatus(text, isError = false) {
    statusEl.textContent = text;
    statusEl.classList.toggle('error', isError);
}

// ── wasm load ──────────────────────────────────────────────────────────

let wasm;
try {
    const { instance } = await WebAssembly.instantiateStreaming(
        fetch('./manifold_csg_playground.wasm'),
        {},
    );
    wasm = instance.exports;
} catch (e) {
    setStatus(`wasm load failed: ${e.message}`, true);
    throw e;
}

// 96-byte scratch (12 × f64) for matrix uploads.
const transformScratchPtr = wasm.alloc(96);

const pushTransform = (slot, matrix4) =>
    pushTransformImpl(wasm, transformScratchPtr, slot, matrix4);
const setPrimitive = (slot, kind) => setPrimitiveImpl(wasm, slot, kind);

// ── three.js scene ────────────────────────────────────────────────────

const scene = new THREE.Scene();
scene.background = new THREE.Color(0x0b0d12);

const camera = new THREE.PerspectiveCamera(50, window.innerWidth / window.innerHeight, 0.01, 100);
camera.position.set(2.4, 1.8, 3.2);

const renderer = new THREE.WebGLRenderer({ antialias: true });
renderer.setPixelRatio(window.devicePixelRatio);
renderer.setSize(window.innerWidth, window.innerHeight);
document.body.appendChild(renderer.domElement);

scene.add(new THREE.AmbientLight(0xffffff, 0.5));
const keyLight = new THREE.DirectionalLight(0xffffff, 1.2);
keyLight.position.set(3, 5, 2);
scene.add(keyLight);
const rimLight = new THREE.DirectionalLight(0x7aa2ff, 0.6);
rimLight.position.set(-3, -2, -2);
scene.add(rimLight);

const grid = new THREE.GridHelper(10, 20, 0x303642, 0x1f242d);
grid.rotation.x = Math.PI / 2; // grid in XY plane (Z up matches manifold's convention)
scene.add(grid);

const orbit = new OrbitControls(camera, renderer.domElement);
orbit.enableDamping = true;
orbit.target.set(0, 0, 0);

// ── Slot proxies (carry the gizmos) ───────────────────────────────────

function makeProxyGeometry(kind) {
    switch (kind) {
        case KIND_CUBE:     return new THREE.BoxGeometry(1.0, 1.0, 1.0);
        case KIND_SPHERE:   return new THREE.SphereGeometry(0.7, 24, 16);
        case KIND_CYLINDER: {
            // manifold's cylinder is along Z; three's CylinderGeometry is
            // along Y, so rotate. Wrap in a group so the gizmo target is
            // a single Object3D.
            const g = new THREE.CylinderGeometry(0.5, 0.5, 1.0, 24);
            g.rotateX(Math.PI / 2);
            return g;
        }
        // Menger sponge occupies the same unit-cube bounding box as a cube;
        // a wireframe box is sufficient for gizmo manipulation. The actual
        // fractal is rendered from the wasm-side rebuild() output.
        case KIND_MENGER:   return new THREE.BoxGeometry(1.0, 1.0, 1.0);
    }
    return new THREE.BoxGeometry(1, 1, 1);
}

const proxyMaterial = new THREE.MeshBasicMaterial({
    color: 0x7aa2ff,
    transparent: true,
    opacity: 0.0,           // invisible by default; shows on hover via wire helper
    depthWrite: false,
});

function makeSlot(slotIdx, initialKind, initialPos) {
    const proxy = new THREE.Mesh(makeProxyGeometry(initialKind), proxyMaterial);
    proxy.userData.slot = slotIdx;
    proxy.userData.kind = initialKind;
    proxy.position.copy(initialPos);
    scene.add(proxy);

    // Wireframe child to visualise the input shape (so the user can see
    // what they're dragging even though the proxy itself is invisible).
    const wire = new THREE.LineSegments(
        new THREE.EdgesGeometry(proxy.geometry),
        new THREE.LineBasicMaterial({
            color: slotIdx === 0 ? 0x7aa2ff : 0xff8a65,
            transparent: true,
            opacity: 0.35,
        }),
    );
    proxy.add(wire);
    proxy.userData.wire = wire;

    return proxy;
}

const slotA = makeSlot(0, KIND_MENGER, new THREE.Vector3(0, 0, 0));
const slotB = makeSlot(1, KIND_SPHERE, new THREE.Vector3(0.5, 0, 0));
const slots = [slotA, slotB];

// ── Result mesh ───────────────────────────────────────────────────────

const resultMaterial = new THREE.MeshStandardMaterial({
    color: 0xeef0f4,
    roughness: 0.4,
    metalness: 0.05,
    flatShading: true,
    side: THREE.DoubleSide,
});

const resultMesh = new THREE.Mesh(new THREE.BufferGeometry(), resultMaterial);
scene.add(resultMesh);

function rebuildAndUpload() {
    const triCount = rebuildIntoMesh(THREE, wasm, resultMesh);
    setStatus(triCount > 0
        ? `boolean result: ${triCount} triangles`
        : 'boolean result: empty');
}

// Coalesce many gizmo events into a single rebuild per animation frame.
let rebuildPending = false;
function scheduleRebuild() {
    if (rebuildPending) return;
    rebuildPending = true;
    requestAnimationFrame(() => {
        rebuildPending = false;
        rebuildAndUpload();
    });
}

// ── Transform gizmo ────────────────────────────────────────────────────

let activeSlot = 0;
const gizmo = new TransformControls(camera, renderer.domElement);
gizmo.size = 0.8;
const gizmoHelper = gizmo.getHelper ? gizmo.getHelper() : gizmo;
scene.add(gizmoHelper);
gizmo.attach(slots[activeSlot]);

gizmo.addEventListener('dragging-changed', (event) => {
    orbit.enabled = !event.value;
});
gizmo.addEventListener('objectChange', () => {
    const obj = gizmo.object;
    if (!obj) return;
    obj.updateMatrix();
    pushTransform(obj.userData.slot, obj.matrix);
    scheduleRebuild();
});

function setActiveSlot(idx) {
    activeSlot = idx;
    gizmo.attach(slots[idx]);
    document.querySelectorAll('.target-btn').forEach((b) => {
        b.classList.toggle('active', Number(b.dataset.target) === idx);
    });
}

function setGizmoMode(mode) {
    gizmo.setMode(mode);
    document.querySelectorAll('.mode-btn').forEach((b) => {
        b.classList.toggle('active', b.dataset.mode === mode);
    });
}

document.querySelectorAll('.target-btn').forEach((b) => {
    b.addEventListener('click', () => setActiveSlot(Number(b.dataset.target)));
});
document.querySelectorAll('.mode-btn').forEach((b) => {
    b.addEventListener('click', () => setGizmoMode(b.dataset.mode));
});

// ── Primitive + op pickers ─────────────────────────────────────────────

function swapProxyShape(slotIdx, kind) {
    const proxy = slots[slotIdx];
    proxy.userData.kind = kind;
    proxy.geometry.dispose();
    proxy.geometry = makeProxyGeometry(kind);
    proxy.userData.wire.geometry.dispose();
    proxy.userData.wire.geometry = new THREE.EdgesGeometry(proxy.geometry);
}

function onPrimitiveChange(slotIdx, kind) {
    setPrimitive(slotIdx, kind);
    swapProxyShape(slotIdx, kind);
    scheduleRebuild();
}

document.getElementById('kind-a').addEventListener('change', (e) => {
    onPrimitiveChange(0, Number(e.target.value));
});
document.getElementById('kind-b').addEventListener('change', (e) => {
    onPrimitiveChange(1, Number(e.target.value));
});
document.getElementById('op').addEventListener('change', (e) => {
    wasm.set_op(Number(e.target.value));
    scheduleRebuild();
});

// ── Initial sync (wasm has its own defaults; mirror them) ─────────────

wasm.set_op(OP_INTERSECTION);
setPrimitive(0, KIND_MENGER);
setPrimitive(1, KIND_SPHERE);
slots[0].updateMatrix();
slots[1].updateMatrix();
pushTransform(0, slots[0].matrix);
pushTransform(1, slots[1].matrix);
rebuildAndUpload();

// ── Render loop ────────────────────────────────────────────────────────

window.addEventListener('resize', () => {
    camera.aspect = window.innerWidth / window.innerHeight;
    camera.updateProjectionMatrix();
    renderer.setSize(window.innerWidth, window.innerHeight);
});

function animate() {
    requestAnimationFrame(animate);
    orbit.update();
    renderer.render(scene, camera);
}
animate();
