"""Reproduce the appearance mount-limit oracle repair in the isolated checkout."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time


BASE = "245ba7838e58318a2d851857b6c2b2d7c757aa09"
TEST = "crates/protocol-oteryn/src/appearance_tests/mappings_tests.rs"
PRODUCTION = "crates/protocol-oteryn/src/appearance/catalogue.rs"


def main():
    parser = argparse.ArgumentParser()
    for name in ["worktree", "evidence", "candidate-json", "target"]:
        parser.add_argument(f"--{name}", type=Path, required=True)
    args = parser.parse_args()
    root, evidence = args.worktree.resolve(), args.evidence.resolve()
    assert subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip() == BASE
    baseline = subprocess.check_output(["git", "show", f"{BASE}:{TEST}"], cwd=root, text=True)
    production = (root / PRODUCTION).read_text()
    assert production == subprocess.check_output(["git", "show", f"{BASE}:{PRODUCTION}"], cwd=root, text=True)
    candidate = json.loads(args.candidate_json.read_text())
    assert set(candidate) == {TEST}
    evidence.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, CARGO_TARGET_DIR=str(args.target), CARGO_BUILD_JOBS="2")
    report = {"baseline": BASE, "rust_toolchain": "1.100.0", "target": "aarch64-unknown-linux-gnu", "runs": []}

    def run(label, command, fails=False):
        started = time.monotonic()
        result = subprocess.run(command, cwd=root, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=900)
        (evidence / f"{label}.log").write_text(result.stdout)
        record = {"label": label, "command": command, "returncode": result.returncode, "seconds": round(time.monotonic() - started, 3), "summaries": re.findall(r"test result: .*", result.stdout)}
        report["runs"].append(record)
        (evidence / "results.json").write_text(json.dumps(report, indent=2) + "\n")
        print(json.dumps(record), flush=True)
        assert result.returncode == (101 if fails else 0), result.stdout[-4000:]
        if fails:
            assert "test result: FAILED" in result.stdout

    guard = "            || self.mounts.len() > 256\n"
    assert production.count(guard) == 1
    try:
        (root / PRODUCTION).write_text(production.replace(guard, "", 1))
        for phase, source, fails in [("baseline", baseline, False), ("candidate", candidate[TEST], True)]:
            (root / TEST).write_text(source)
            run(f"{phase}_mutant_mount_limit", ["cargo", "+1.100.0", "test", "--offline", "--locked", "--quiet", "-p", "oteryn-protocol-oteryn", "--lib", "appearance_catalogue_palette_and_list_bounds_are_symmetric"], fails)
    finally:
        (root / TEST).write_text(candidate[TEST])
        (root / PRODUCTION).write_text(production)
    run("format_changed", ["rustfmt", "+1.100.0", "--config-path", "rustfmt.toml", TEST])
    report["candidate_sha256"] = {TEST: hashlib.sha256((root / TEST).read_bytes()).hexdigest()}
    report["production_restored_sha256"] = {PRODUCTION: hashlib.sha256((root / PRODUCTION).read_bytes()).hexdigest()}
    (evidence / "formatted-candidate-files.json").write_text(json.dumps({TEST: (root / TEST).read_text()}))
    run("candidate_unit", ["cargo", "+1.100.0", "test", "--offline", "--locked", "--quiet", "-p", "oteryn-protocol-oteryn", "--lib"])
    run("candidate_clippy", ["cargo", "+1.100.0", "clippy", "--offline", "--locked", "--quiet", "-p", "oteryn-protocol-oteryn", "--all-targets", "--", "-D", "warnings"])
    run("format_changed_check", ["rustfmt", "+1.100.0", "--config-path", "rustfmt.toml", "--check", TEST])
    run("candidate_diff_check", ["git", "diff", "--check"])


if __name__ == "__main__":
    main()
