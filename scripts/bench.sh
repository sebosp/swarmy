#!/bin/bash
# Runs the benchmarks of the queries (src-tauri/benches/queries.rs) on a snapshot and compares
# them with the committed baseline (scripts/bench-compare.py): before a release, on corvid.
#
#   scripts/bench.sh                                  # ~/SC2Replays/PRO, compare with the baseline
#   scripts/bench.sh --accept                         # ... and make this run the baseline
#   SWARMY_TEST_REPLAYS=~/SCReplaysOnNVMe scripts/bench.sh   # the full snapshot
#   scripts/bench.sh matchups                         # one query, no comparison
#
# Steady numbers: the Polars thread pool is pinned to POLARS_MAX_THREADS (16, the P-cores of
# corvid's i9-13900KF, CPUs 0-15), the process to those CPUs with taskset when it exists, and the
# first pass warms the page cache. The medians of ten samples are compared, 10% is noise.
set -euo pipefail
cd "$(dirname "$0")/.."
export SWARMY_TEST_REPLAYS="${SWARMY_TEST_REPLAYS:-$HOME/SC2Replays/PRO}"
export POLARS_MAX_THREADS="${POLARS_MAX_THREADS:-16}"
accept=
filter=()
for arg in "$@"; do
  case "$arg" in
    --accept) accept=1 ;;
    *) filter+=("$arg") ;;
  esac
done
pin=()
if command -v taskset >/dev/null && [ "$(nproc)" -ge 24 ]; then
  pin=(taskset -c 0-15)
fi
echo "=== snapshot $SWARMY_TEST_REPLAYS, $POLARS_MAX_THREADS Polars threads${pin:+, CPUs 0-15}"
"${pin[@]}" cargo bench -p swarmy --bench queries -- "${filter[@]}"
if [ ${#filter[@]} -gt 0 ]; then
  echo "=== one query, no comparison with the baseline"
  exit 0
fi
history=
[ -d "$HOME/ci-runs" ] && history="--history $HOME/ci-runs/bench-history.jsonl"
# shellcheck disable=SC2086
scripts/bench-compare.py ${accept:+--accept} $history
