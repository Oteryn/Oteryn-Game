"""Run the actual content test methods with controlled in-memory mutants only."""
import argparse
import ast
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
import unittest


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def result(case):
    observed = unittest.TestResult()
    case.run(observed)
    return {
        "tests_run": observed.testsRun,
        "failures": [trace for _, trace in observed.failures],
        "errors": [trace for _, trace in observed.errors],
        "skips": [reason for _, reason in observed.skipped],
        "accepted": observed.wasSuccessful(),
    }


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--repo", type=Path, required=True)
parser.add_argument("--output", type=Path, required=True)
args = parser.parse_args()
root = args.repo.resolve()
migration = root / "tools/content-migration"
sys.path.insert(0, str(migration))
coin = load("coin_tests", migration / "test_coin_converter_profiles.py")
coin.CoinConverterProfiles.setUpClass()
original_ids = set(coin.converters.IDS)
coins = {}
for name, ids in [("current_source", original_ids), ("one_id_only", {23722}), ("empty_ids", set())]:
    coin.converters.IDS = ids
    coins[name] = result(coin.CoinConverterProfiles("test_converters_are_carried_nonstackable_tools_with_500_charges"))
coin.converters.IDS = original_ids

rating = load("rating_tests", migration / "test_creature_level_rating.py")
rating.CreatureLevelRatingTests.setUpClass()
calibration = copy.deepcopy(rating.CreatureLevelRatingTests.report["calibration"])
variants = {
    "current_source": calibration,
    "empty_calibration": [],
    "missing_one_area": calibration[:-1],
    "duplicate_area": calibration + [copy.deepcopy(calibration[0])],
}
wrong_observed = copy.deepcopy(calibration)
wrong_observed[0]["observed_tibia"] += 1
variants["changed_reference_value"] = wrong_observed
wrong_rating = copy.deepcopy(calibration)
wrong_rating[0]["oteryn"] = wrong_rating[0]["observed_tibia"] * 2
variants["out_of_tolerance_control"] = wrong_rating
ratings = {}
for name, rows in variants.items():
    case = rating.CreatureLevelRatingTests("test_calibration_within_ten_percent")
    case.report = {"calibration": rows}
    ratings[name] = result(case)

engine_path = root / "tools/content-schema/item-authoring/test_engine_items.py"
parsed = ast.parse(engine_path.read_bytes())
defined = {node.name for node in parsed.body if isinstance(node, ast.FunctionDef) and node.name.startswith("test_")}
main = next(node for node in parsed.body if isinstance(node, ast.FunctionDef) and node.name == "main")
listing = next(node.value for node in main.body if isinstance(node, ast.Assign) and any(isinstance(target, ast.Name) and target.id == "tests" for target in node.targets))
selected = [node.id for node in listing.elts]
record = {
    "mode": "actual Python test methods; in-memory mutations, no repository/game/content writes",
    "coin": coins,
    "calibration": ratings,
    "calibration_source_values": calibration,
    "engine_registration": {
        "defined_functions": len(defined),
        "selected_functions": len(selected),
        "unique_selected": len(set(selected)),
        "missing": sorted(defined - set(selected)),
    },
    "source_sha256": {str(path.relative_to(root)): hashlib.sha256(path.read_bytes()).hexdigest() for path in [
        migration / "test_coin_converter_profiles.py",
        migration / "coin_converter_profiles.py",
        migration / "test_creature_level_rating.py",
        migration / "creature_level_rating.py",
        engine_path,
    ]},
}
args.output.parent.mkdir(parents=True, exist_ok=True)
args.output.write_text(json.dumps(record, indent=2) + "\n")
print(json.dumps({"coin_accepts": {key: value["accepted"] for key, value in coins.items()}, "calibration_accepts": {key: value["accepted"] for key, value in ratings.items()}, "engine_registration": record["engine_registration"]}, indent=2))
