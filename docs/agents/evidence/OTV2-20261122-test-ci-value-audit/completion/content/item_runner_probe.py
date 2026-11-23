#!/usr/bin/env python3
"""Witness the actual manual runner's selected identities and Frost Cannon oracle.

Identity recording substitutes only top-level test callables and the three sibling
unittest suites. It is selection evidence, not execution of the 108 test bodies.
The focused Frost Cannon body and its production-route mutant are executed for real.
"""
import argparse
import hashlib
import importlib
import io
import json
from pathlib import Path
import sys
from contextlib import redirect_stdout, redirect_stderr
from unittest.mock import patch
import unittest


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    source = args.repo.resolve() / "tools/content-schema/item-authoring"
    sys.path.insert(0, str(source))
    module = importlib.import_module("test_engine_items")
    defined = {name: value for name, value in vars(module).items()
               if name.startswith("test_") and callable(value)
               and getattr(value, "__module__", None) == module.__name__}
    selected = []
    sentinels = {name: (lambda name=name: selected.append(name)) for name in defined}
    with patch.multiple(module, **sentinels), patch.object(
            module.unittest.defaultTestLoader, "discover", return_value=unittest.TestSuite()), \
            redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
        module.main()
    focus = defined["test_frost_cannon_routes_non_pickupable_blocking_prop"]
    focus()
    with patch.object(module.engine_items, "NON_PICKUPABLE_BLOCKING_PROP_IDS", frozenset()):
        try:
            focus()
        except (AssertionError, KeyError) as error:
            mutant = {"rejected": True, "failure_type": type(error).__name__, "message": str(error)}
        else:
            mutant = {"rejected": False}
    result = {
        "source_sha256": hashlib.sha256((source / "test_engine_items.py").read_bytes()).hexdigest(),
        "defined": sorted(defined), "actual_main_selected": selected,
        "missing": sorted(set(defined) - set(selected)),
        "unexpected": sorted(set(selected) - set(defined)),
        "duplicates": sorted(name for name in set(selected) if selected.count(name) != 1),
        "focused_real_case": "PASS", "empty_production_route_mutant": mutant,
        "selection_probe_is_full_suite_execution": False,
    }
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({key: value for key, value in result.items()
                      if key not in {"defined", "actual_main_selected"}}, indent=2))


if __name__ == "__main__":
    main()
