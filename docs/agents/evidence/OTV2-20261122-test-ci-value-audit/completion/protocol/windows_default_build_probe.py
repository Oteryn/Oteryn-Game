"""Required default package builds for exact candidate hashes; no test rerun."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

BASE = "245ba7838e58318a2d851857b6c2b2d7c757aa09"
PROTECTED = "crates/input-platform/tests/native_winit.rs"
INPUT_SHA = "c80929f7bdf0241f0f6a778449f343de81903f5afb1f559a4a5ccc7c7d71dcce"
PACKAGES = ["oteryn-protocol-oteryn", "oteryn-identity", "oteryn-gameplay-foundation", "oteryn-client-runtime"]
parser = argparse.ArgumentParser()
for name in ["worktree", "evidence", "payload", "target"]:
    parser.add_argument(f"--{name}", type=Path, required=True)
args = parser.parse_args()
root, out = args.worktree.resolve(), args.evidence.resolve()
assert subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root).decode().strip() == BASE
payload = json.loads(args.payload.read_text(encoding="utf-8"))
assert set(payload["candidate"]) == {"crates/identity/src/lib.rs", "crates/gameplay/foundation/src/scheduled_events.rs"}
assert hashlib.sha256((root / PROTECTED).read_bytes()).hexdigest() == INPUT_SHA
for path, digest in payload["retained"].items():
    assert hashlib.sha256((root / path).read_bytes()).hexdigest() == digest, path
for path, value in payload["candidate"].items():
    baseline = subprocess.check_output(["git", "show", f"{BASE}:{path}"], cwd=root)
    assert (root / path).read_bytes().replace(b"\r\n", b"\n") == baseline, path
    (root / path).write_bytes(value.encode("utf-8"))
hashes = dict(payload["retained"], **{p: hashlib.sha256((root / p).read_bytes()).hexdigest() for p in payload["candidate"]})
out.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, CARGO_TARGET_DIR=str(args.target), CARGO_BUILD_JOBS="4")
report = {"baseline": BASE, "rust_toolchain": "1.100.0", "target": "x86_64-pc-windows-msvc", "source_sha256": hashes, "protected_input_sha256": INPUT_SHA, "scope": "Four ordinary default-feature cargo build commands. No no-default-features, feature override, custom RUSTFLAGS or test execution.", "runs": []}


def record():
    (out / "results.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8", newline="\n")


record()
for package in PACKAGES:
    command = ["cargo", "+1.100.0", "build", "--locked", "--quiet", "-p", package]
    start = time.monotonic()
    result = subprocess.run(command, cwd=root, env=env, text=True, encoding="utf-8", errors="replace", stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=1800)
    label = "default_build_" + package
    (out / f"{label}.log").write_text(result.stdout, encoding="utf-8", newline="\n")
    entry = {"label": label, "command": command, "returncode": result.returncode, "seconds": round(time.monotonic() - start, 3)}
    report["runs"].append(entry)
    record()
    print(json.dumps(entry), flush=True)
    assert result.returncode == 0, result.stdout[-6000:]
for path, digest in hashes.items():
    assert hashlib.sha256((root / path).read_bytes()).hexdigest() == digest, path
assert hashlib.sha256((root / PROTECTED).read_bytes()).hexdigest() == INPUT_SHA
report["source_hashes_preserved"] = True
report["protected_input_unchanged"] = True
report["completed"] = True
record()
