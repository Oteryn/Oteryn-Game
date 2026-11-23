"""Bounded isolated proof for the regeneration dispatch test repair."""
from __future__ import annotations

import ast
import contextlib
import inspect
import io
import json
from pathlib import Path
import subprocess
import sys
import time
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[6]
sys.path.insert(0, str(ROOT / "tools/content-migration"))
import test_regenerate_content as tests
import test_world_project_v2_to_tree as inventory


def result(call):
    try:
        call()
    except Exception as error:
        return {"status": "REJECTED", "exception": type(error).__name__, "message": str(error)}
    return {"status": "PASS"}


report = {
    "baseline": json.loads(Path(__file__).with_name("regenerate_boundary_before.json").read_text()),
    "repaired_focus": result(tests.test_source_cohort_includes_installed_successors),
}
original = inspect.getsource(tests.rc.main)
mutants = {
    "omit_buddel_installer": original.replace("    install_buddel(ROOT, WORLD)\n", ""),
    "measure_before_buddel_installer": original.replace(
        "    install_buddel(ROOT, WORLD)\n", "    refresh_source_cohort_extent()\n    install_buddel(ROOT, WORLD)\n"),
}
report["mutants"] = {}
for name, code in mutants.items():
    assert code != original
    namespace = dict(tests.rc.__dict__)
    exec(compile(code, "<regenerate-dispatch-mutant>", "exec"), namespace)
    # The function must share the real module globals so the test boundary's
    # temporary paths and producer traps still apply to the in-memory mutant.
    import types
    function = types.FunctionType(namespace["main"].__code__, tests.rc.__dict__)
    with patch.object(tests.rc, "main", function):
        report["mutants"][name] = result(tests.test_source_cohort_includes_installed_successors)

source = Path(tests.__file__).read_text()
discovered = sorted(node.name for node in ast.parse(source).body
                    if isinstance(node, ast.FunctionDef) and node.name.startswith("test_"))
selected = []
with contextlib.ExitStack() as stack, contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
    for name in discovered:
        stack.enter_context(patch.object(tests, name, side_effect=lambda name=name: selected.append(name)))
    tests.main()
report["manual_runner_selection"] = {
    "discovered": discovered, "executed_order": selected,
    "missing": sorted(set(discovered) - set(selected)),
    "extra": sorted(set(selected) - set(discovered)),
    "duplicates": sorted(name for name in set(selected) if selected.count(name) != 1),
    "proof": "Each discovered test callable replaced with an identity recorder; the actual main() dispatches all 36 exactly once. Real execution is recorded separately below.",
}
assert sorted(selected) == discovered
checks = [check for check in inventory.check_inventory(workflow_checks=True)
          if any(str(part).endswith("/tools/content-migration/test_regenerate_content.py")
                 for part in check.command)]
assert len(checks) == 1, checks
check = checks[0]
command = list(check.command)
command[0] = sys.executable
start = time.monotonic()
completed = subprocess.run(command, cwd=check.cwd, text=True, capture_output=True, timeout=120)
report["existing_inventory_execution"] = {
    "name": check.name, "command": command, "cwd": str(check.cwd or Path.cwd()),
    "exit_code": completed.returncode, "stdout": completed.stdout,
    "stderr": completed.stderr, "seconds": round(time.monotonic() - start, 3),
}
Path(__file__).with_name("regenerate_boundary_after.json").write_text(json.dumps(report, indent=2) + "\n")
assert report["repaired_focus"]["status"] == "PASS"
assert all(row["status"] == "REJECTED" and row["exception"] == "AssertionError"
           for row in report["mutants"].values())
assert completed.returncode == 0, completed.stderr[-4000:]
print(json.dumps({"focus": report["repaired_focus"], "mutants": report["mutants"],
                  "selected": len(selected), "actual_exit": completed.returncode,
                  "actual_stdout_tail": completed.stdout[-500:]}, indent=2))
