"""Qualify root-owned client assertions in the separate Windows validation copy."""

import difflib
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time

ROOT = Path(r"C:\Users\barte\work\oteryn-test-value-20261123")
OUT = Path(r"C:\Users\barte\work\oteryn-protocol-audit-20261123\client-followup")
BASE = "245ba7838e58318a2d851857b6c2b2d7c757aa09"
GUARD = "crates/client-runtime/src/features/0049-house/protocol.rs"
PROTECTED = "crates/input-platform/tests/native_winit.rs"
payload = json.loads(Path(r"C:\Users\barte\work\oteryn-protocol-audit-20261123\client-followup-payload.json").read_text(encoding="utf-8"))
paths = {*payload, GUARD}
baseline = {p: subprocess.check_output(["git", "show", f"{BASE}:{p}"], cwd=ROOT).decode("utf-8") for p in paths}
assert subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT).decode().strip() == BASE
for path in paths:
    assert (ROOT / path).read_bytes().replace(b"\r\n", b"\n") == baseline[path].encode("utf-8"), path
protected_hash = hashlib.sha256((ROOT / PROTECTED).read_bytes()).hexdigest()
OUT.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, CARGO_TARGET_DIR=r"C:\Users\barte\work\oteryn-test-value-20261123-target", CARGO_BUILD_JOBS="4")
report = {"baseline": BASE, "platform": "x86_64-pc-windows-msvc", "rust_toolchain": "1.100.0", "protected_input_sha256": protected_hash, "runs": []}

def write(path, value):
    (ROOT / path).write_bytes(value.encode("utf-8"))

def record():
    (OUT / "results.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8", newline="\n")

def run(label, command, fail=False, tests=False):
    start = time.monotonic()
    result = subprocess.run(command, cwd=ROOT, env=env, text=True, encoding="utf-8", errors="replace", stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=1800)
    (OUT / f"{label}.log").write_text(result.stdout, encoding="utf-8", newline="\n")
    entry = {"label": label, "command": command, "returncode": result.returncode, "seconds": round(time.monotonic() - start, 3), "summaries": re.findall(r"test result: .*", result.stdout)}
    report["runs"].append(entry)
    record()
    print(json.dumps(entry), flush=True)
    assert result.returncode == (101 if fail else 0), result.stdout[-5000:]
    if fail:
        assert "test result: FAILED" in result.stdout
    elif tests:
        assert any(int(n) > 0 for n in re.findall(r"test result: ok\. (\d+) passed", result.stdout))

final = dict(baseline, **payload)
try:
    for phase, source, fails in [("baseline", baseline, False), ("candidate", final, True)]:
        for path, value in source.items():
            write(path, value)
        guard = '        require(members.len() <= 200, "At most 200 members")?;\n'
        assert source[GUARD].count(guard) == 1
        write(GUARD, source[GUARD].replace(guard, "", 1))
        run(f"{phase}_mutant_house_member_guard", ["cargo", "+1.100.0", "test", "--locked", "--quiet", "-p", "oteryn-client-runtime", "--test", "gap_house", "gap_house_members_and_largest_door_request_stay_bounded"], fail=fails, tests=True)
finally:
    for path, value in final.items():
        write(path, value)

normalized = []
for path in subprocess.check_output(["git", "ls-files", "crates/client-runtime/*.rs"], cwd=ROOT).decode().splitlines():
    if path in payload:
        continue
    canonical = subprocess.check_output(["git", "show", f"{BASE}:{path}"], cwd=ROOT)
    current = (ROOT / path).read_bytes()
    assert current.replace(b"\r\n", b"\n") == canonical, path
    if current != canonical:
        (ROOT / path).write_bytes(canonical)
        normalized.append(path)
report["canonicalized_unchanged_paths"] = normalized
run("format_changed", ["rustfmt", "+1.100.0", "--config-path", "rustfmt.toml", *payload])
formatted = {p: (ROOT / p).read_text(encoding="utf-8") for p in payload}
patch = "".join("".join(difflib.unified_diff(payload[p].splitlines(True), formatted[p].splitlines(True), fromfile=f"a/{p}", tofile=f"b/{p}")) for p in payload)
(OUT / "formatting.patch").write_text(patch, encoding="utf-8", newline="\n")
(OUT / "formatted-files.json").write_text(json.dumps(formatted), encoding="utf-8", newline="\n")
report["candidate_sha256"] = {p: hashlib.sha256((ROOT / p).read_bytes()).hexdigest() for p in payload}
report["production_guard_restored"] = (ROOT / GUARD).read_bytes() == baseline[GUARD].encode("utf-8")
assert report["production_guard_restored"]
run("candidate_unit", ["cargo", "+1.100.0", "test", "--locked", "--quiet", "-p", "oteryn-client-runtime"], tests=True)
run("candidate_clippy", ["cargo", "+1.100.0", "clippy", "--locked", "--quiet", "-p", "oteryn-client-runtime", "--all-targets", "--", "-D", "warnings"])
run("format_package_check", ["cargo", "+1.100.0", "fmt", "--check", "-p", "oteryn-client-runtime"])
run("candidate_diff_check", ["git", "diff", "--check", "--", "crates/client-runtime"])
assert hashlib.sha256((ROOT / PROTECTED).read_bytes()).hexdigest() == protected_hash
report["protected_input_unchanged"] = True
report["completed"] = True
record()
