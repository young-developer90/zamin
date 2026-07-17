#!/usr/bin/env bash
set -euo pipefail

# Build the Zamin WebAssembly runtime for the browser playground.
#
# Prerequisites:
#   - Rust wasm32 target: rustup target add wasm32-unknown-unknown
#   - wasm-pack: cargo install wasm-pack
#   - wasm-bindgen (installed automatically by wasm-pack)
#
# Usage:
#   ./build-wasm.sh
#
# Output is placed in docs/wasm/ and can be served as static files
# alongside the rest of the documentation website.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

echo "==> Building Zamin WASM runtime..."

wasm-pack build \
    --target web \
    --out-dir docs/wasm \
    --out-name zamin_wasm \
    --features wasm \
    --no-default-features \
    .

echo ""
echo "==> Build complete!"
echo "    Output: docs/wasm/"
echo "    Files:"
ls -lh docs/wasm/
echo ""
echo "==> To test, serve docs/ with a static HTTP server,"
echo "    then open the playground page in your browser."
echo ""
echo "    Example:"
echo "      python3 -m http.server -d docs/ 8000"
echo "      # open http://localhost:8000/playground.html"
