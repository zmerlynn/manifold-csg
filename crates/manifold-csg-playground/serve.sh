#!/usr/bin/env bash
# Serve web/ on http://localhost:8000 so the browser can fetch the .wasm
# (file:// URLs can't run WebAssembly.instantiateStreaming in most browsers).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PORT="${PORT:-8000}"

cd "$SCRIPT_DIR/web"
echo "serving $SCRIPT_DIR/web on http://localhost:$PORT"
exec python3 -m http.server "$PORT"
