#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
TARGET="x86_64-unknown-linux-gnu"
if [[ "${1:-}" != "--no-build" ]]; then
  cargo build --release --locked --target "$TARGET"
fi
BINARY="target/$TARGET/release/battlestar_voxel"
[[ -f "$BINARY" ]] || { echo "Native Linux binary missing: $BINARY" >&2; exit 1; }
python3 script/package_native.py --platform linux --target "$TARGET" --binary "$BINARY"
