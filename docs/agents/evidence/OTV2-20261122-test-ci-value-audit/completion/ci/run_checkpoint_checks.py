"""Run the repository-owned checkpoint commands and preserve individual exits."""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
import time
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root = args.repo.resolve()
    commands = [
        [sys.executable, "tools/repository/validate_repository_policy.py"],
        [sys.executable, "tools/repository/refresh_ci_workflow_pins.py", "--check"],
        [sys.executable, "tools/agents/validate_governance.py"],
        [sys.executable, "-m", "unittest", "discover", "-s", "tools/agents/tests"],
    ]
    commands.extend([sys.executable, str(path.relative_to(root))]
                    for path in sorted((root / "tools/repository").glob("test_*.py")))
    commands.extend([
        ["bash", "-n", "tools/qualification/node_boot/run.sh"],
        ["bash", "-n", "tools/qualification/wp5_s3b/run.sh"],
        ["git", "diff", "--check"],
    ])
    record = {
        "scope": "Actual repository entrypoints, governance, pins, shell syntax and whitespace; no full product/PG/Windows/GPU claim.",
        "source_head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip(),
        "python": sys.version,
        "commands": [],
        "changed_source_sha256": {},
        "redaction": "Output lines containing password, database URLs, secret, token, key or grant are omitted; SHA256 binds the complete raw output.",
    }
    paths = subprocess.check_output(["git", "diff", "--name-only", "-z"], cwd=root).decode().split("\0")
    for path in filter(None, paths):
        if (root / path).is_file():
            record["changed_source_sha256"][path] = hashlib.sha256((root / path).read_bytes()).hexdigest()
    sensitive = re.compile(r"password|postgres(?:ql)?://|mysql://|secret|token|key|grant", re.I)
    for command in commands:
        start = time.monotonic()
        result = subprocess.run(command, cwd=root, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        raw = result.stdout
        lines = raw.splitlines()
        retained = [line for line in lines if not sensitive.search(line)]
        item = {
            "command": command,
            "exit_code": result.returncode,
            "elapsed_seconds": round(time.monotonic() - start, 3),
            "output": "\n".join(retained),
            "raw_output_sha256": hashlib.sha256(raw.encode()).hexdigest(),
            "redacted_lines": len(lines) - len(retained),
        }
        record["commands"].append(item)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(record, indent=2) + "\n")
        print(json.dumps({k: item[k] for k in ("command", "exit_code", "elapsed_seconds")}), flush=True)
    record["all_commands_passed"] = all(item["exit_code"] == 0 for item in record["commands"])
    args.output.write_text(json.dumps(record, indent=2) + "\n")
    return 0 if record["all_commands_passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
