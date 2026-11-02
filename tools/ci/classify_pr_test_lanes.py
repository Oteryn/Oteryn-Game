#!/usr/bin/env python3
"""Expose the trusted lane classifier's Cargo reverse closure to the PR Rust job."""
from __future__ import annotations

import importlib.util
import json
import os
from pathlib import Path
import sys

SPEC = importlib.util.spec_from_file_location(
    "trusted_lanes", Path(__file__).resolve().parents[1] / "repository/classify_pr_test_lanes.py"
)
lanes = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(lanes)


def select(files, changed_count, metadata, complete=True):
    """Unknown evidence or any unowned input selects workspace; prose selects none."""
    workspace = {"mode": "workspace", "packages": []}
    try:
        roots, reverse = lanes.graph(metadata)
        workspace["packages"] = sorted(roots)
        if (complete is not True or not isinstance(files, list) or not files
                or type(changed_count) is not int or len(files) != changed_count):
            return workspace
        selected = set()
        for record in files:
            if not isinstance(record, dict) or record.get("status") not in {
                "added", "modified", "removed", "renamed", "copied", "changed", "unchanged"
            }:
                return workspace
            if record["status"] in {"renamed", "copied"} and not record.get("previous_filename"):
                return workspace
            paths = [record.get("filename")]
            if "previous_filename" in record:
                paths.append(record["previous_filename"])
            for path in paths:
                if not lanes.valid_path(path):
                    return workspace
                owner = lanes.package_owner(roots, path)
                if owner:
                    selected.add(owner)
                elif lanes.documentation_path(path):
                    continue
                else:
                    return workspace
        # The same implementation used for PostgreSQL/Windows lane reachability;
        # includes optional, dev, build and target-specific workspace dependencies.
        lanes.expand_reverse_closure(selected, reverse)
        return {"mode": "workspace" if selected == set(roots) else "packages" if selected else "none",
                "packages": sorted(selected)}
    except (KeyError, TypeError, ValueError, AttributeError):
        return workspace


def main():
    try:
        metadata = json.loads(Path(sys.argv[1]).read_text())
        if os.environ.get("ENUMERATION_COMPLETE") == "true":
            files, count, complete = lanes.pr_file_records()
        elif os.environ.get("ENUMERATION_COMPLETE") == "false":
            files = lanes.git_diff_records(os.environ["EXPECTED_BASE"], os.environ["EXPECTED_HEAD"])
            count, complete = len(files), True
        else:
            raise ValueError("invalid enumeration state")
        selection = select(files, count, metadata, complete)
    except (OSError, ValueError, KeyError, TypeError, lanes.subprocess.SubprocessError):
        selection = {"mode": "workspace", "packages": []}
    # A prose file can still be a build input (include_str!, build.rs, etc.).
    # The trusted lane classifier already scans those consumers: preserve its decision.
    if selection["mode"] == "none" and os.environ.get("RUST_REQUIRED") == "true":
        selection = {"mode": "workspace", "packages": sorted(lanes.graph(metadata)[0])}
    print(json.dumps(selection, sort_keys=True))


if __name__ == "__main__":
    main()
