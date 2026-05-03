#!/usr/bin/env bash
# Build the playground wasm + run the Node-side tests.
#
# The wasm tests load web/manifold_csg_playground.wasm directly, so the
# build must run first. The rebuild tests need real three.js, installed
# via npm into ./node_modules (gitignored).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

bash "$SCRIPT_DIR/build.sh"

cd "$SCRIPT_DIR"
if [ ! -d node_modules ]; then
    echo "→ installing test deps (one-time)..."
    npm install --no-audit --no-fund
fi

node --test tests/*.test.mjs
