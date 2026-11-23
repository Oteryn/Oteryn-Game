"""Finish Unicode export qualification after replacing the denied expect assertion."""

import argparse
import difflib
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time

BASE = "245ba7838e58318a2d851857b6c2b2d7c757aa09"
TEST = "crates/client-ui/tests/chat_log.rs"
PRODUCTION = "crates/client-ui/src/features/0007-chat/view.rs"
parser = argparse.ArgumentParser()
for name in ["worktree", "evidence", "payload", "target"]:
    parser.add_argument(f"--{name}", type=Path, required=True)
args = parser.parse_args()
root, out = args.worktree.resolve(), args.evidence.resolve()
assert subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root).decode().strip() == BASE
report = json.loads((out / "results.json").read_text())
payload = json.loads(args.payload.read_text())
assert set(payload) == {TEST}
assert hashlib.sha256((root / TEST).read_bytes()).hexdigest() == report["candidate_sha256"][TEST]
baseline = subprocess.check_output(["git", "show", f"{BASE}:{PRODUCTION}"], cwd=root)
assert (root / PRODUCTION).read_bytes() == baseline
for path, digest in report["protected_source_sha256"].items():
    assert hashlib.sha256((root / path).read_bytes()).hexdigest() == digest, path
report["initial_candidate_sha256"] = report["candidate_sha256"]
env = dict(os.environ, CARGO_TARGET_DIR=str(args.target), CARGO_BUILD_JOBS="2")


def record():
    (out / "results.json").write_text(json.dumps(report, indent=2) + "\n")


def run(label, command, fail=False, tests=False):
    start = time.monotonic()
    result = subprocess.run(command, cwd=root, env=env, text=True, encoding="utf-8", errors="replace", stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=1800)
    (out / f"{label}.log").write_text(result.stdout)
    entry = {"label": label, "command": command, "returncode": result.returncode, "seconds": round(time.monotonic() - start, 3), "summaries": re.findall(r"test result: .*", result.stdout)}
    report["runs"].append(entry)
    record()
    print(json.dumps(entry), flush=True)
    assert result.returncode == (101 if fail else 0), result.stdout[-5000:]
    if fail:
        assert "test result: FAILED" in result.stdout
    elif tests:
        assert any(int(n) > 0 for n in re.findall(r"test result: ok\. (\d+) passed", result.stdout))


old = b"    writer.write_all(panel_text(panel).as_bytes())?;\n"
new = b"    writer.write_all(panel_text(panel).chars().filter(|c| c.is_ascii()).collect::<String>().as_bytes())?;\n"
assert baseline.count(old) == 1
try:
    (root / TEST).write_bytes(payload[TEST].encode())
    (root / PRODUCTION).write_bytes(baseline.replace(old, new, 1))
    report["final_mutation_candidate_sha256"] = {TEST: hashlib.sha256((root / TEST).read_bytes()).hexdigest()}
    run("candidate_mutant_ascii_only_log_final", ["cargo", "test", "--locked", "--quiet", "-p", "oteryn-client-ui", "--test", "chat_log"], fail=True, tests=True)
finally:
    (root / PRODUCTION).write_bytes(baseline)
    (root / TEST).write_bytes(payload[TEST].encode())
run("format_changed_final", ["rustfmt", "--config-path", "rustfmt.toml", TEST])
formatted = (root / TEST).read_text()
(out / "formatting-final.patch").write_text("".join(difflib.unified_diff(payload[TEST].splitlines(True), formatted.splitlines(True), fromfile=f"a/{TEST}", tofile=f"b/{TEST}")))
report["candidate_sha256"] = {TEST: hashlib.sha256((root / TEST).read_bytes()).hexdigest()}
report["production_restored"] = (root / PRODUCTION).read_bytes() == baseline
assert report["production_restored"]
run("candidate_unit_final", ["cargo", "test", "--locked", "--quiet", "-p", "oteryn-client-ui"], tests=True)
run("candidate_clippy_final", ["cargo", "clippy", "--locked", "--quiet", "-p", "oteryn-client-ui", "--all-targets", "--", "-D", "warnings"])
run("candidate_build_final", ["cargo", "build", "--locked", "--quiet", "-p", "oteryn-client-ui"])
run("format_package_check_final", ["cargo", "fmt", "--check", "-p", "oteryn-client-ui"])
run("candidate_diff_check_final", ["git", "diff", "--check", "--", "crates/client-ui"])
for path, digest in report["protected_source_sha256"].items():
    assert hashlib.sha256((root / path).read_bytes()).hexdigest() == digest, path
report["other_changed_sources_preserved"] = True
report["resolved_initial_clippy_failure"] = "Initial candidate_clippy failed on denied expect_used in chat_log.rs. Root replaced expect/contains with is_ok_and; final candidate mutation and package qualification cover the recorded final SHA."
report["completed"] = True
record()
