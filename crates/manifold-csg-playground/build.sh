#!/usr/bin/env bash
# Build the playground wasm and copy it into web/.
#
# Requirements (same as the wasm32-unknown-unknown target):
#   - rustup target add wasm32-unknown-unknown
#   - LLVM 20+ on PATH or via WASM_CXX_SHIM_LLVM_BIN_DIR
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

cp "$WASM" "$DEST"
echo "wrote $DEST ($(wc -c < "$DEST") bytes)"
