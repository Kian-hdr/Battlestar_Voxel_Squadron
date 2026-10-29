#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p evidence
MODE="${1:-run}"
case "$MODE" in run|--debug|--logs|--telemetry|--verify|--smoke-test|--input-check) ;; *) echo "usage: $0 [--debug|--logs|--telemetry|--verify|--smoke-test|--input-check]" >&2; exit 2;; esac
# Match this project's packaged executable, preserving unrelated processes.
for BUNDLE in Battlestar_Voxel Battlestar_Voxel_Squadron; do
  pkill -f "$PWD/dist/$BUNDLE.app/Contents/MacOS/Battlestar_Voxel" >/dev/null 2>&1 || true
done
./script/package_mac.sh
APP="$PWD/dist/Battlestar_Voxel_Squadron.app"
case "$MODE" in
 --debug) lldb -- "$APP/Contents/MacOS/Battlestar_Voxel" ;;
 --logs|--telemetry) open -n "$APP" --stdout "$PWD/evidence/runtime.log" --stderr "$PWD/evidence/runtime.log"; tail -f evidence/runtime.log ;;
 --smoke-test) EVIDENCE_DIR="${BSVX_EVIDENCE_DIR:-$PWD/evidence}"; mkdir -p "$EVIDENCE_DIR"; : > "$EVIDENCE_DIR/smoke.log"; rm -f "$EVIDENCE_DIR/Smoke_Result.json"; open -n "$APP" --env "BSVX_EVIDENCE_DIR=$EVIDENCE_DIR" --env RUST_LOG=info --stdout "$EVIDENCE_DIR/smoke.log" --stderr "$EVIDENCE_DIR/smoke.log" --args --smoke-test ;;
 --input-check) EVIDENCE_DIR="${BSVX_EVIDENCE_DIR:-$PWD/evidence}"; mkdir -p "$EVIDENCE_DIR"; : > "$EVIDENCE_DIR/native_input.log"; open -n "$APP" --env BSVX_INPUT_TRACE=1 --env BSVX_SILENT_CHECK=1 --env RUST_LOG=info --stdout "$EVIDENCE_DIR/native_input.log" --stderr "$EVIDENCE_DIR/native_input.log" ;;
 --verify) open -n "$APP"; sleep 2; pgrep -f "$APP/Contents/MacOS/Battlestar_Voxel" >/dev/null ;;
 *) open -n "$APP" ;;
esac
