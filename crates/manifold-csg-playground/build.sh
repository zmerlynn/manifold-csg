#!/usr/bin/env bash
# Build the playground wasm and copy it into web/.
#
# Requirements (same as the wasm32-unknown-unknown target):
#   - rustup target add wasm32-unknown-unknown
#   - LLVM 20+ on PATH or via WASM_CXX_SHIM_LLVM_BIN_DIR
#
# Optional:
#   - wasm-opt (binaryen) for ~20% smaller artifacts. `brew install
#     binaryen` or `apt install binaryen`. The Pages workflow installs
#     it; local devs without it get a slightly larger wasm but the
#     same functionality.
#
# See ../../docs/plans/wasm-unknown-unknown.md and the README.md "Browser
# without Emscripten" section for the toolchain setup details.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$WORKSPACE_ROOT"
cargo build --target wasm32-unknown-unknown --release -p manifold-csg-playground

WASM="$WORKSPACE_ROOT/target/wasm32-unknown-unknown/release/manifold_csg_playground.wasm"
DEST="$SCRIPT_DIR/web/manifold_csg_playground.wasm"

if command -v wasm-opt >/dev/null 2>&1; then
    BEFORE=$(wc -c < "$WASM")
    wasm-opt -Oz "$WASM" -o "$DEST"
    AFTER=$(wc -c < "$DEST")
    echo "wrote $DEST ($BEFORE → $AFTER bytes, $(( (BEFORE - AFTER) * 100 / BEFORE ))% smaller via wasm-opt -Oz)"
else
    cp "$WASM" "$DEST"
    echo "wrote $DEST ($(wc -c < "$DEST") bytes — install binaryen for ~20% smaller artifact)"
fi
