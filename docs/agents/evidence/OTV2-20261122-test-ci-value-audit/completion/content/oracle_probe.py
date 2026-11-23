#!/usr/bin/env python3
"""Run the actual affected tests and controlled in-memory oracle mutants.

The mutants alter only the test interpreter, never source or content files.
An AssertionError is expected from every mutant after the test-oracle repair.
"""
import argparse
import copy
import hashlib
import importlib
import io
import json
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch


def run_suite(suite):
    output = io.StringIO()
    result = unittest.TextTestRunner(stream=output, verbosity=2).run(suite)
    return {"run": result.testsRun, "failures": len(result.failures),
            "errors": len(result.errors), "skipped": len(result.skipped),
            "successful": result.wasSuccessful(), "output": output.getvalue()}


def invoke(case, method):
    try:
        getattr(case(method), method)()
    except AssertionError as error:
        return {"rejected": True, "error": str(error)}
    except Exception as error:
        return {"rejected": False, "unexpected_error": repr(error)}
    return {"rejected": False}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    repo = args.repo.resolve()
    source = repo / "tools/content-migration"
    sys.path.insert(0, str(source))
    names = ["test_coin_converter_profiles", "test_creature_level_rating",
             "test_boss_corpse_terminal_reference", "test_cake_bridge_reference"]
    modules = {name: importlib.import_module(name) for name in names}
    report = {
        "head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repo, text=True).strip(),
        "source_sha256": {name: hashlib.sha256((source / (name + ".py")).read_bytes()).hexdigest()
                          for name in names},
        "unmodified_inputs": run_suite(unittest.TestSuite(
            unittest.defaultTestLoader.loadTestsFromModule(module) for module in modules.values())),
        "mutants": {}, "direct": {}}
    coin = modules["test_coin_converter_profiles"]
    coin.CoinConverterProfiles.setUpClass()
    for label, ids in [("coin_singleton", {23722}), ("coin_empty", set())]:
        with patch.object(coin.converters, "IDS", ids):
            report["mutants"][label] = invoke(coin.CoinConverterProfiles,
                "test_converters_are_carried_nonstackable_tools_with_500_charges")
    rating = modules["test_creature_level_rating"]
    rating.CreatureLevelRatingTests.setUpClass()
    actual = rating.CreatureLevelRatingTests.report
    rows = actual["calibration"]
    changed = copy.deepcopy(rows)
    changed[0]["observed_tibia"] += 1
    for label, calibration in [("calibration_empty", []), ("calibration_missing", rows[:-1]),
                               ("calibration_duplicate", rows + [rows[0]]),
                               ("calibration_wrong_reference", changed)]:
        with patch.object(rating.CreatureLevelRatingTests, "report", {**actual, "calibration": calibration}):
            report["mutants"][label] = invoke(rating.CreatureLevelRatingTests,
                                               "test_calibration_within_ten_percent")
    for name in ("test_boss_corpse_terminal_reference", "test_cake_bridge_reference"):
        result = subprocess.run([sys.executable, str(source / (name + ".py")), "-v"],
                                cwd=source, capture_output=True, text=True)
        report["direct"][name] = {"exit_code": result.returncode,
                                  "stdout": result.stdout, "stderr": result.stderr}
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"unmodified_inputs": {k: v for k, v in report["unmodified_inputs"].items() if k != "output"},
                      "mutants": report["mutants"], "direct": report["direct"]}, indent=2))


if __name__ == "__main__":
    main()
