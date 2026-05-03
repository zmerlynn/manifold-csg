// crates/manifold-csg/wasm32-uu-runner/run.mjs
//
// Loads the wasm32-unknown-unknown smoke example and invokes its
// exported `smoke_run`. Asserts a positive triangle count. The exact
// value isn't load-bearing — what matters is that manifold's CSG kernel
// actually executes when called from a wasm32-unknown-unknown module
// linked against wasm-cxx-shim. A zero or negative return means
// something's broken (allocation failure, dead-stripped function call,
// etc.).
//
// Usage:
//   cargo build --example wasm32_uu_smoke --target wasm32-unknown-unknown \
//       -p manifold-csg --no-default-features
//   node crates/manifold-csg/wasm32-uu-runner/run.mjs \
//       target/wasm32-unknown-unknown/debug/examples/wasm32_uu_smoke.wasm

import fs from 'node:fs';
import process from 'node:process';

if (process.argv.length < 3) {
    console.error('usage: node run.mjs <path-to-wasm32_uu_smoke.wasm>');
    process.exit(2);
}

const bytes = fs.readFileSync(process.argv[2]);
const { instance } = await WebAssembly.instantiate(bytes, {});

if (typeof instance.exports.smoke_run !== 'function') {
    console.error('wasm32-uu-smoke: wasm has no exported smoke_run');
    console.error('exports:', Object.keys(instance.exports));
    process.exit(1);
}

const got = instance.exports.smoke_run();
if (!Number.isInteger(got) || got <= 0) {
    console.error(
        `wasm32-uu-smoke: smoke_run returned ${got}; expected positive integer (triangle count)`
    );
    process.exit(1);
}

console.log(`wasm32-uu-smoke: smoke_run() = ${got} (triangle count, > 0) ✓`);
