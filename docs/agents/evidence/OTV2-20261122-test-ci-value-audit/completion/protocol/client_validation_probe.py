"""Qualify the root agent's client changes in the disposable VPS checkout.

The two negative synthetic cases alter harness outputs/inputs, not library
implementations. This verifies the synthetic CLI oracle, not end-to-end gameplay.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time


BASE = "245ba7838e58318a2d851857b6c2b2d7c757aa09"
HARNESS = "tools/synthetic-client-harness/src/main.rs"
PATHS = [
    "crates/input-platform/tests/native_winit.rs",
    "crates/client-runtime/src/features/0064-target-move/tests/e2e_m01.rs",
    HARNESS,
]


def mutate(text, name):
    if name == "drop_actions":
        marker = "    let generation = ProcessGeneration::new(1);"
        assert text.count(marker) == 1
        return text.replace(marker, "    let actions: Vec<_> = actions.into_iter().take(0).collect();\n\n" + marker)
    start = text.index("    simulation.apply(ProjectionEvent::Upsert(")
    end = text.index("    let image =", start)
    return text[:start] + "    simulation.apply(ProjectionEvent::Remove(ClientEntityRef::new(2)))?;\n\n" + text[end:]


def main():
    parser = argparse.ArgumentParser()
    for name in ["worktree", "evidence", "candidate-json", "target"]:
        parser.add_argument(f"--{name}", type=Path, required=True)
    args = parser.parse_args()
    root, evidence = args.worktree.resolve(), args.evidence.resolve()
    assert subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip() == BASE
    candidate = json.loads(args.candidate_json.read_text())
    assert set(candidate) == set(PATHS)
    baseline = subprocess.check_output(["git", "show", f"{BASE}:{HARNESS}"], cwd=root, text=True)
    evidence.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, CARGO_TARGET_DIR=str(args.target), CARGO_BUILD_JOBS="2")
    report = {"baseline": BASE, "rust_toolchain": "1.100.0", "target": "aarch64-unknown-linux-gnu", "scope": "synthetic CLI; no live server or native window", "runs": []}

    def run(label, command, expected=0, required=None):
        started = time.monotonic()
        result = subprocess.run(command, cwd=root, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=1200)
        (evidence / f"{label}.log").write_text(result.stdout)
        record = {"label": label, "command": command, "returncode": result.returncode, "seconds": round(time.monotonic() - started, 3), "summaries": re.findall(r"(?:test result: .*|synthetic-ok .*|Error: .*)", result.stdout)}
        report["runs"].append(record)
        (evidence / "results.json").write_text(json.dumps(report, indent=2) + "\n")
        print(json.dumps(record), flush=True)
        assert result.returncode == expected, result.stdout[-4000:]
        assert required is None or required in result.stdout, result.stdout[-4000:]

    for path, text in candidate.items():
        (root / path).write_text(text)
    run("format_changed", ["rustfmt", "+1.100.0", "--config-path", "rustfmt.toml", *PATHS])
    candidate = {p: (root / p).read_text() for p in PATHS}
    report["candidate_sha256"] = {p: hashlib.sha256((root / p).read_bytes()).hexdigest() for p in PATHS}
    (evidence / "formatted-candidate-files.json").write_text(json.dumps(candidate))
    run("client_runtime_unit", ["cargo", "+1.100.0", "test", "--locked", "--quiet", "-p", "oteryn-client-runtime"])
    run("client_runtime_clippy", ["cargo", "+1.100.0", "clippy", "--offline", "--locked", "--quiet", "-p", "oteryn-client-runtime", "--all-targets", "--", "-D", "warnings"])
    cli = ["cargo", "+1.100.0", "run", "--locked", "--quiet", "-p", "oteryn-synthetic-client-harness"]
    try:
        for phase, source in [("baseline", baseline), ("candidate", candidate[HARNESS])]:
            (root / HARNESS).write_text(source)
            run(f"{phase}_synthetic", cli, required="synthetic-ok revision=1 entities=2 asset=checker actions=1")
            for name, error in [("drop_actions", "unexpected synthetic input action"), ("omit_target", "unexpected synthetic world projection")]:
                (root / HARNESS).write_text(mutate(source, name))
                run(f"{phase}_synthetic_{name}", cli, expected=0 if phase == "baseline" else 1, required="synthetic-ok" if phase == "baseline" else error)
    finally:
        for path, text in candidate.items():
            (root / path).write_text(text)
    run("harness_build", ["cargo", "+1.100.0", "build", "--offline", "--locked", "--quiet", "-p", "oteryn-synthetic-client-harness"])
    run("harness_clippy", ["cargo", "+1.100.0", "clippy", "--locked", "--quiet", "-p", "oteryn-synthetic-client-harness", "--all-targets", "--", "-D", "warnings"])
    run("format_changed_check", ["rustfmt", "+1.100.0", "--config-path", "rustfmt.toml", "--check", *PATHS])
    run("candidate_diff_check", ["git", "diff", "--check"])


if __name__ == "__main__":
    main()
