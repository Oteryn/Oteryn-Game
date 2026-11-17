#!/usr/bin/env python3
"""Exercise the real seed CLI's conflict path without forwarding private output."""
import json
import os
import subprocess
import sys
from pathlib import Path


def main():
    work = Path(sys.argv[1])
    env = json.loads((work / "portal/secrets/env.json").read_text())
    env = {**os.environ, **env, "OTERYN_ENV": "testing"}
    args = json.loads((work / "route-input.json").read_text())
    original = args.copy()
    args["port"] = args["port"] % 65535 + 1
    binary = Path(json.loads((work / "portal-meta.json").read_text())["target_directory"]) / "debug/oteryn-portal"
    result = subprocess.run([str(binary), "seed", "route", json.dumps(args)],
                            env=env, capture_output=True, timeout=15, check=False)
    if result.returncode != 2 or json.loads(result.stdout).get("error") != "conflict":
        raise ValueError("route_conflict_not_refused")
    replay = subprocess.run([str(binary), "seed", "route", json.dumps(original)],
                            env=env, capture_output=True, timeout=15, check=False)
    expected = json.loads((work / "portal-report.json").read_text())["seed"]["route"]
    if replay.returncode != 0 or json.loads(replay.stdout) != expected:
        raise ValueError("route_conflict_changed_record")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, KeyError, TypeError, subprocess.TimeoutExpired):
        print("route_seed_check_failed", file=sys.stderr)
        sys.exit(1)
