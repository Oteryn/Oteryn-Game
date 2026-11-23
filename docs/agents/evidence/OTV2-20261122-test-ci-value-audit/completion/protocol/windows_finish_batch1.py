"""Finish batch 1 after canonicalizing two unchanged embedded schema fixtures."""

import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time

ROOT = Path(r"C:\Users\barte\work\oteryn-test-value-20261123")
OUT = Path(r"C:\Users\barte\work\oteryn-protocol-audit-20261123\batch1")
BASE = "245ba7838e58318a2d851857b6c2b2d7c757aa09"
report = json.loads((OUT / "results.json").read_text())
env = dict(os.environ, CARGO_TARGET_DIR=r"C:\Users\barte\work\oteryn-test-value-20261123-target", CARGO_BUILD_JOBS="4")
assert subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT).decode().strip() == BASE
for path, digest in report["candidate_sha256"].items():
    assert hashlib.sha256((ROOT / path).read_bytes()).hexdigest() == digest, path
for path, digest in report["restored_production_sha256"].items():
    assert hashlib.sha256((ROOT / path).read_bytes()).hexdigest() == digest, path
protected = ROOT / "crates/input-platform/tests/native_winit.rs"
assert hashlib.sha256(protected.read_bytes()).hexdigest() == report["protected_input_sha256"]
canonicalized = []
for path in [
    "docs/contracts/protocol-oteryn/v1/world_clock_v1.proto",
    "docs/contracts/protocol-oteryn/v1/item_view_v1.proto",
]:
    canonical = subprocess.check_output(["git", "show", f"{BASE}:{path}"], cwd=ROOT)
    current = (ROOT / path).read_bytes()
    assert current.replace(b"\r\n", b"\n") == canonical, path
    if current != canonical:
        (ROOT / path).write_bytes(canonical)
        canonicalized.append(path)
report["canonicalized_unchanged_schema_fixtures"] = canonicalized
report["initial_unit_failure"] = "Two unchanged embedded .proto fixtures had checkout CRLF; tests require canonical LF substrings. Their bytes normalized to LF exactly matched baseline before replacement."

for label, command in [
    ("candidate_unit_canonical_fixtures", ["cargo", "+1.100.0", "test", "--locked", "--quiet", "-p", "oteryn-protocol-oteryn", "--lib"]),
    ("candidate_clippy", ["cargo", "+1.100.0", "clippy", "--locked", "--quiet", "-p", "oteryn-protocol-oteryn", "--all-targets", "--", "-D", "warnings"]),
    ("format_package_check", ["cargo", "+1.100.0", "fmt", "--check", "-p", "oteryn-protocol-oteryn"]),
    ("candidate_diff_check", ["git", "diff", "--check", "--", "crates/protocol-oteryn"]),
]:
    started = time.monotonic()
    result = subprocess.run(command, cwd=ROOT, env=env, text=True, encoding="utf-8", errors="replace", stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=1200)
    (OUT / f"{label}.log").write_text(result.stdout, encoding="utf-8", newline="\n")
    entry = {"label": label, "command": command, "returncode": result.returncode, "seconds": round(time.monotonic() - started, 3), "summaries": re.findall(r"test result: .*", result.stdout)}
    report["runs"].append(entry)
    (OUT / "results.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(json.dumps(entry), flush=True)
    assert result.returncode == 0, result.stdout[-4000:]
    if "unit" in label:
        assert "911 passed; 0 failed" in result.stdout
assert hashlib.sha256(protected.read_bytes()).hexdigest() == report["protected_input_sha256"]
report["protected_input_unchanged"] = True
report["completed"] = True
(OUT / "results.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8", newline="\n")
