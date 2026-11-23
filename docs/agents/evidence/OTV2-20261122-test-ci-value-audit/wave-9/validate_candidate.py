#!/usr/bin/env python3
"""Run the actual affected tools and retain bounded, reproducible evidence."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--phase", required=True)
    args = parser.parse_args()
    repo = args.repo.resolve()
    manifest = json.loads(args.manifest.read_text())
    for entry in manifest["files"]:
        path = repo / entry["path"]
        if entry.get("deleted"):
            assert not path.exists(), entry["path"]
        else:
            assert hashlib.sha256(path.read_bytes()).hexdigest() == entry["sha256"], entry["path"]
    log_dir = args.output.parent / (args.output.stem + "_logs")
    log_dir.mkdir(exist_ok=True)
    result = {
        "phase": args.phase,
        "started_at": datetime.now(timezone.utc).isoformat(),
        "repo": str(repo),
        "head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repo, text=True).strip(),
        "python": sys.version,
        "source_manifest_sha256": hashlib.sha256(args.manifest.read_bytes()).hexdigest(),
        "source_hashes_verified": len(manifest["files"]),
        "checks": [],
        "not_run": ["full Game/Platform/PostgreSQL topology", "Windows/GPU", "complete remaining semantic audit"],
    }
    python = sys.executable
    specs = [("repository/" + path.stem, [python, str(path.relative_to(repo))], repo)
             for path in sorted((repo / "tools/repository").glob("test_*.py"))]
    result["repository_entrypoints"] = len(specs)
    specs.extend([
        ("node_boot_discovery", [python, "-m", "unittest", "discover", "-s", "tools/qualification/node_boot", "-p", "test_*.py"], repo),
        ("agent_governance_tests", [python, "-m", "unittest", "discover", "-s", "tools/agents/tests"], repo),
        ("repository_policy", [python, "tools/repository/validate_repository_policy.py"], repo),
        ("agent_governance", [python, "tools/agents/validate_governance.py"], repo),
        ("workflow_pins", [python, "tools/repository/refresh_ci_workflow_pins.py", "--check"], repo),
        ("content_migration_focused", [python, "-m", "unittest", "-v", "test_boss_corpse_terminal_reference", "test_coin_converter_profiles", "test_creature_level_rating", "test_parallel_checks"], repo / "tools/content-migration"),
        ("item_engine", [python, "tools/content-schema/item-authoring/test_engine_items.py"], repo),
        ("node_boot_shell_syntax", ["bash", "-n", "tools/qualification/node_boot/run.sh"], repo),
        ("s3b_shell_syntax", ["bash", "-n", "tools/qualification/wp5_s3b/run.sh"], repo),
        ("diff_check", ["git", "diff", "--check"], repo),
    ])
    for name, command, cwd in specs:
        print("START", name, flush=True)
        started = time.monotonic()
        completed = subprocess.run(command, cwd=cwd, env=os.environ.copy(), text=True, capture_output=True)
        raw = completed.stdout + "\n--- STDERR ---\n" + completed.stderr
        log = log_dir / (name.replace("/", "__") + ".log")
        log.write_text(raw)
        entry = {
            "name": name, "argv": command, "cwd": str(cwd),
            "exit_code": completed.returncode, "seconds": round(time.monotonic() - started, 3),
            "unittest_counts": [int(count) for count in re.findall(r"Ran (\d+) tests? in ", completed.stderr)],
            "log_path": str(log), "log_sha256": hashlib.sha256(log.read_bytes()).hexdigest(),
            "tail": raw[-2500:],
        }
        result["checks"].append(entry)
        args.output.write_text(json.dumps(result, indent=2) + "\n")
        print("PASS" if completed.returncode == 0 else "FAIL", name, completed.returncode, entry["seconds"], flush=True)
    protected = [".github/workflows/agent-governance.yml", ".github/workflows/merge-gate.yml", ".github/workflows/merge-group-gate.yml"]
    result["protected_workflow_blobs"] = {}
    for name in protected:
        baseline = subprocess.check_output(["git", "show", manifest["parent"] + ":" + name], cwd=repo)
        assert (repo / name).read_bytes() == baseline, name
        result["protected_workflow_blobs"][name] = subprocess.check_output(["git", "hash-object", name], cwd=repo, text=True).strip()
    result["finished_at"] = datetime.now(timezone.utc).isoformat()
    result["failed"] = [check["name"] for check in result["checks"] if check["exit_code"] != 0]
    result["passed"] = len(result["checks"]) - len(result["failed"])
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({key: result[key] for key in ("head", "phase", "repository_entrypoints", "passed", "failed")}))
    return bool(result["failed"])


if __name__ == "__main__":
    raise SystemExit(main())
