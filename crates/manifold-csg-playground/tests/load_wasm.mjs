// Shared wasm loader for Node-side tests.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));

export async function loadWasm() {
    const wasmPath = join(__dirname, '..', 'web', 'manifold_csg_playground.wasm');
    const bytes = readFileSync(wasmPath);
    const { instance } = await WebAssembly.instantiate(bytes, {});
    return instance.exports;
}
