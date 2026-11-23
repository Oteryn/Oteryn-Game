"""Execute only the three repaired modules through the real migration check plan."""
import argparse
import importlib.util
import json
from pathlib import Path
import sys
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--repo", type=Path, required=True)
parser.add_argument("--output", type=Path, required=True)
args = parser.parse_args()
root = args.repo.resolve()
path = root / "tools/content-migration/test_world_project_v2_to_tree.py"
sys.path.insert(0, str(path.parent))
spec = importlib.util.spec_from_file_location("migration_checks", path)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
wanted = {
    "test_boss_corpse_terminal_reference",
    "test_coin_converter_profiles",
    "test_creature_level_rating",
}
plan = module.check_inventory(workflow_checks=True)
selected = [check for check in plan if check.name in wanted]
assert {check.name for check in selected} == wanted
assert len(selected) == len(wanted)
started = time.monotonic()
module.run_checks(selected, jobs=2)
record = {
    "full_workflow_plan_count": len(plan),
    "executed_selected_checks": [
        {"name": check.name, "argv": check.command, "cwd": str(check.cwd)}
        for check in selected
    ],
    "seconds": round(time.monotonic() - started, 3),
    "all_selected_passed": True,
    "scope": "Three added complete modules only, through real check_inventory and parallel_checks.run_checks; remaining existing plan not re-executed here.",
}
args.output.parent.mkdir(parents=True, exist_ok=True)
args.output.write_text(json.dumps(record, indent=2) + "\n")
