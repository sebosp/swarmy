#!/usr/bin/env python3
"""Compares the medians of `cargo bench --bench queries` with the committed baseline.

Reads criterion's `target/criterion/<group>/<query>/<parameter>/new/estimates.json`, prints a
Markdown table (query, baseline, now, change) and fails (exit 1) when a query is slower than the
baseline by more than FAIL_PERCENT; a change under WARN_PERCENT either way is noise and shown as
"same". The medians are in milliseconds. The parameter (the map, the player) is part of the id:
another snapshot is another baseline.

    scripts/bench-compare.py                      # compare with benches/baseline-corvid.json
    scripts/bench-compare.py --accept             # write the baseline from this run
    scripts/bench-compare.py --baseline other.json --history ~/ci-runs/bench-history.jsonl

The history file gets one JSON line per run (host, time, revision, medians): to plot the trend.
See src-tauri/benches/queries.rs and scripts/bench.sh.
"""

import argparse
import datetime
import json
import os
import pathlib
import socket
import subprocess
import sys

WARN_PERCENT = 10.0
FAIL_PERCENT = 20.0

REPO = pathlib.Path(__file__).resolve().parent.parent


def medians(criterion_dir: pathlib.Path) -> dict[str, float]:
    """The median of each bench, in ms, by `group/query/parameter`."""
    result = {}
    for estimates in sorted(criterion_dir.glob("**/new/estimates.json")):
        relative = estimates.relative_to(criterion_dir).parent.parent
        median_ns = json.loads(estimates.read_text())["median"]["point_estimate"]
        result[relative.as_posix()] = median_ns / 1e6
    return result


def revision() -> str:
    # A build copy without .git (the deploy script's) says the commit in the environment.
    if sha := os.environ.get("SWARMY_GIT_SHA"):
        return sha[:9] + ("-dirty" if os.environ.get("SWARMY_GIT_DIRTY") == "1" else "")
    try:
        sha = subprocess.run(
            ["git", "rev-parse", "--short", "HEAD"], capture_output=True, text=True, check=True, cwd=REPO
        ).stdout.strip()
        dirty = subprocess.run(["git", "status", "--porcelain"], capture_output=True, text=True, cwd=REPO).stdout
        return sha + ("-dirty" if dirty.strip() else "")
    except (OSError, subprocess.CalledProcessError):
        return "unknown"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--criterion", default=REPO / "target" / "criterion", type=pathlib.Path)
    parser.add_argument("--baseline", default=REPO / "src-tauri" / "benches" / "baseline-corvid.json", type=pathlib.Path)
    parser.add_argument("--accept", action="store_true", help="write this run as the baseline")
    parser.add_argument("--history", type=pathlib.Path, help="append a JSON line of this run to the file")
    args = parser.parse_args()

    now = medians(args.criterion)
    if not now:
        print(f"no results in {args.criterion}: run scripts/bench.sh first", file=sys.stderr)
        return 2
    if args.history:
        line = {
            "host": socket.gethostname(),
            "time": datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds"),
            "revision": revision(),
            "median_ms": now,
        }
        args.history.parent.mkdir(parents=True, exist_ok=True)
        with args.history.open("a") as history:
            history.write(json.dumps(line, sort_keys=True) + "\n")
    if args.accept:
        args.baseline.write_text(
            json.dumps(
                {
                    "host": socket.gethostname(),
                    "revision": revision(),
                    "snapshot": os.environ.get("SWARMY_TEST_REPLAYS", ""),
                    "median_ms": now,
                },
                indent=1,
                sort_keys=True,
            )
            + "\n"
        )
        print(f"baseline written: {args.baseline} ({len(now)} queries)")
        return 0
    if not args.baseline.exists():
        print(f"no baseline at {args.baseline}: run with --accept once", file=sys.stderr)
        return 2
    baseline = json.loads(args.baseline.read_text())["median_ms"]

    print("| Query | Baseline ms | Now ms | Change |")
    print("|---|---:|---:|---|")
    failed = []
    for name in sorted(set(baseline) | set(now)):
        before, after = baseline.get(name), now.get(name)
        if before is None:
            print(f"| `{name}` | - | {after:.1f} | new |")
            continue
        if after is None:
            print(f"| `{name}` | {before:.1f} | - | missing |")
            failed.append(f"{name}: not run")
            continue
        change = (after - before) / before * 100
        if abs(change) < WARN_PERCENT:
            verdict = "same"
        elif change < 0:
            verdict = f"{-change:.0f}% faster"
        elif change < FAIL_PERCENT:
            verdict = f"{change:.0f}% slower, warning"
        else:
            verdict = f"**{change:.0f}% slower**"
            failed.append(f"{name}: {before:.1f} -> {after:.1f} ms")
        print(f"| `{name}` | {before:.1f} | {after:.1f} | {verdict} |")
    if failed:
        print(f"\n{len(failed)} quer{'y is' if len(failed) == 1 else 'ies are'} slower than the baseline by more than {FAIL_PERCENT:.0f}%:")
        for line in failed:
            print(f"  {line}")
        print("Fix the query, or accept the new times with --accept and say why in the commit.")
        return 1
    print(f"\nNo query slower than the baseline by {FAIL_PERCENT:.0f}% or more.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
