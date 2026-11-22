#!/usr/bin/env python3
"""Read-only, synthetic audit reproducers. Never starts the game or a database."""
import argparse
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import tempfile


def baseline_repro(repo):
    source = (repo / "tools/qualification/node_boot/run.sh").read_text()
    helpers = []
    for name in ("stage_rows", "known_rows"):
        helpers.append(re.search(rf"(?m)^{name}\(\).*$", source).group(0))
    strict = re.search(r"(?ms)^stage_fail_rows\(\).*?^}", source)
    if strict:
        helpers.append(strict.group(0))
    helpers.append(re.search(r"(?ms)^stage\(\).*?^}", source).group(0))
    with tempfile.TemporaryDirectory() as tmp:
        work = Path(tmp)
        (work / "vars").write_text("")
        (work / "node.log").write_text("")
        script = r'''
set -Eeuo pipefail
WORK="$1"; GAME_SOURCE="$2"; VARS="$WORK/vars"; node_log="$WORK/node.log"
NODE_BOOT_WORLD=1; NODE_BOOT_SPELLS=0; declare -A STATUS=(); ORDER=()
evidence() { :; }
start_runtime_diagnostics() { :; }
stop_runtime_diagnostics() { :; }
pg_observability_dump() { :; }
synthetic_row() {
  printf '%s\n' 'SEAM_EVIDENCE e2e_all scenario=E01.full status=FAIL seconds=1 evidence=synthetic-new-regression cause=assertion root=E01/relog-bank-balance'
}
'''
        script += "\n".join(helpers)
        script += '\nstage audit -- synthetic_row\nprintf "AUDIT_STAGE_STATUS=%s\\n" "${STATUS[audit]}"\n'
        result = subprocess.run(["bash", "-c", script, "audit", str(work), str(repo)],
                                text=True, capture_output=True, timeout=15)
        status = re.search(r"(?m)^AUDIT_STAGE_STATUS=(.+)$", result.stdout)
        return {"kind": "baseline_signature", "process_exit": result.returncode,
                "known_root": "E01/greet", "observed_root": "E01/relog-bank-balance",
                "actual_stage_status": status[1] if status else None,
                "output": result.stdout, "stderr": result.stderr}


def aggregate_repro(repo):
    path = repo / "tools/qualification/node_boot/matrix.py"
    spec = importlib.util.spec_from_file_location("audit_node_boot_matrix", path)
    matrix = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(matrix)
    plan = matrix.plan("custom", rows="206:207")
    with tempfile.TemporaryDirectory() as tmp:
        directory = Path(tmp)
        log = directory / "qualification.log"
        log.write_text("SEAM_EVIDENCE e2e_all scenario=I01.full status=PASS seconds=1\n"
                       "NODE_BOOT_RESULT=NODE_BOOT_PASS\n")
        shard = plan[0]
        output = directory / shard["name"]
        output.mkdir()
        result = matrix.extract(log, name=shard["name"], entry=shard["entry"],
                                exit_code=0, seconds=3)
        (output / "result.json").write_text(json.dumps(result))
        markdown, problems = matrix.summarize(plan, directory, "")
        return {"kind": "aggregate_scenario_identity", "requested_batch": "206:1",
                "expected_from_audited_dispatch": "S09.client", "observed": "I01.full PASS",
                "shard_passed": matrix.shard_passed(result), "summary_problems": problems,
                "markdown": markdown}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--mode", choices=("all", "baseline", "aggregate"), default="all")
    args = parser.parse_args()
    repo = args.repo.resolve()
    results = []
    if args.mode in ("all", "baseline"):
        results.append(baseline_repro(repo))
    if args.mode in ("all", "aggregate"):
        results.append(aggregate_repro(repo))
    print(json.dumps({"source_directory": str(repo), "synthetic_reproducers": results}, indent=2))


if __name__ == "__main__":
    main()
