#!/usr/bin/env bash
# Observe a known game PID after a quit request; never terminate it or launch GUI.
set -euo pipefail
if [[ $# -lt 1 || $# -gt 3 || ! "$1" =~ ^[0-9]+$ ]]; then
  echo "usage: $0 GAME_PID [WAIT_SECONDS=15] [EVIDENCE_DIRECTORY]" >&2
  exit 2
fi
GAME_PID="$1"
WAIT_SECONDS="${2:-15}"
if [[ ! "$WAIT_SECONDS" =~ ^[0-9]+$ ]] || (( WAIT_SECONDS > 60 )); then
  echo "WAIT_SECONDS must be an integer from 0 to 60" >&2
  exit 2
fi
EVIDENCE_DIRECTORY="${3:-$(cd "$(dirname "$0")/.." && pwd)/evidence}"
command_name=$(ps -p "$GAME_PID" -o comm= 2>/dev/null || true)
if [[ -z "$command_name" ]]; then
  echo "LIFECYCLE PID $GAME_PID is no longer present"
  exit 0
fi
case "${command_name##*/}" in
  Battlestar_Voxel|battlestar_voxel) ;;
  *) echo "Refusing to sample unrelated PID $GAME_PID ($command_name)" >&2; exit 2 ;;
esac
for ((elapsed=0; elapsed<=WAIT_SECONDS; elapsed++)); do
  state=$(ps -p "$GAME_PID" -o stat= 2>/dev/null || true)
  if [[ -z "$state" ]]; then
    echo "LIFECYCLE PID $GAME_PID exited within $elapsed seconds"
    exit 0
  fi
  if [[ "$state" == *Z* ]]; then
    echo "LIFECYCLE PID $GAME_PID exited but parent has not reaped it (zombie)"
    exit 0
  fi
  if (( elapsed < WAIT_SECONDS )); then sleep 1; fi
done
mkdir -p "$EVIDENCE_DIRECTORY"
stamp=$(date -u +%Y%m%dT%H%M%SZ)
report="$EVIDENCE_DIRECTORY/Lifecycle_${GAME_PID}_${stamp}"
ps -p "$GAME_PID" -o pid,ppid,stat,etime,pcpu,command > "${report}_Process.txt"
if [[ $(uname -s) == Darwin ]]; then
  # Stack sampling distinguishes event-loop wait, rendering, and audio teardown.
  /usr/bin/sample "$GAME_PID" 3 -file "${report}_Sample.txt" || true
fi
echo "LIFECYCLE PID $GAME_PID remained alive; diagnostics: ${report}_*"
echo "Leave the process available for diagnosis; this helper sends no signals."
exit 1
