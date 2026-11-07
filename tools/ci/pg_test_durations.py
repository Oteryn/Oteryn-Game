#!/usr/bin/env python3
"""Build the PR PostgreSQL harness duration estimates from partition job logs.

Reads downloaded `PR PostgreSQL harness` partition logs (nextest PASS/FAIL lines),
keeps the longest observed duration of every test at or above the threshold and
writes them as JSON for the partition planner. A missing or stale entry only
costs partition balance: the planner still runs every listed test exactly once.
"""

import argparse
import json
from pathlib import Path
import re

RESULT = re.compile(r"(?:PASS|FAIL) \[ *([0-9.]+)s\] \( *\d+/ *\d+\) (\S+) (\S+)")


def durations(paths):
    longest = {}
    for path in paths:
        for line in Path(path).read_text(errors="replace").splitlines():
            match = RESULT.search(line)
            if match:
                key = f"{match[2]} {match[3]}"
                longest[key] = max(longest.get(key, 0.0), float(match[1]))
    return longest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("logs", nargs="+", help="partition job log files")
    parser.add_argument("--threshold", type=float, default=30.0, help="seconds")
    parser.add_argument("--out", required=True)
    args = parser.parse_args()
    longest = durations(args.logs)
    if not longest:
        raise SystemExit("no nextest results found in the logs")
    kept = {key: round(seconds) for key, seconds in sorted(longest.items()) if seconds >= args.threshold}
    Path(args.out).write_text(json.dumps(kept, indent=1, sort_keys=True) + "\n")
    print(f"{len(kept)} of {len(longest)} tests at or above {args.threshold:g}s")


if __name__ == "__main__":
    main()
