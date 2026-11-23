"""Compare independent analyser limit fixtures against removed production guards.

Run after mutation_probe.py in the same disposable baseline worktree. The two
additional candidate files are supplied as JSON; the five earlier candidates
remain untouched. No commit, push or deployment is performed.
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
CATALOGUE = "crates/protocol-oteryn/src/analyser_catalogue.rs"
PARTY = "crates/protocol-oteryn/src/analyser_party.rs"
PATHS = [CATALOGUE, PARTY]
ALL_PATHS = [
    "crates/identity/src/lib.rs",
    "crates/protocol-oteryn/src/daily_boost.rs",
    "crates/protocol-oteryn/src/guild_war.rs",
    "crates/protocol-oteryn/src/contract_registries.rs",
    "crates/gameplay/foundation/src/scheduled_events.rs",
    *PATHS,
]


def mutate(files):
    files = dict(files)
    for path, guard in [
        (CATALOGUE, "        && ids.len() <= MAX_ITEMS\n"),
        (PARTY, '    if prices.len() > MAX_PRICES {\n        return Err("too many party prices");\n    }\n'),
    ]:
        assert files[path].count(guard) == 1
        files[path] = files[path].replace(guard, "", 1)
    return files


def main():
    parser = argparse.ArgumentParser()
    for name in ["worktree", "evidence", "candidate-json", "target"]:
        parser.add_argument(f"--{name}", type=Path, required=True)
    args = parser.parse_args()
    root, evidence = args.worktree.resolve(), args.evidence.resolve()
    assert subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip() == BASE
    baseline = {p: subprocess.check_output(["git", "show", f"{BASE}:{p}"], cwd=root, text=True) for p in PATHS}
    candidate = json.loads(args.candidate_json.read_text())
    assert set(candidate) == set(PATHS)
    evidence.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, CARGO_TARGET_DIR=str(args.target), CARGO_BUILD_JOBS="2")
    report = {"baseline": BASE, "rust_toolchain": "1.100.0", "target": "aarch64-unknown-linux-gnu", "runs": []}

    def write(files):
        for path, text in files.items():
            (root / path).write_text(text)

    def run(label, command, fails=False):
        started = time.monotonic()
        result = subprocess.run(command, cwd=root, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=900)
        (evidence / f"{label}.log").write_text(result.stdout)
        record = {"label": label, "command": command, "returncode": result.returncode, "seconds": round(time.monotonic() - started, 3), "summaries": re.findall(r"test result: .*", result.stdout)}
        report["runs"].append(record)
        (evidence / "results.json").write_text(json.dumps(report, indent=2) + "\n")
        print(json.dumps(record), flush=True)
        if fails:
            assert result.returncode == 101 and "test result: FAILED" in result.stdout, result.stdout[-4000:]
        else:
            assert result.returncode == 0, result.stdout[-4000:]

    try:
        for phase, files, fails in [("baseline_mutant", baseline, False), ("candidate_mutant", candidate, True)]:
            write(mutate(files))
            for name in ["analyser_catalogue", "analyser_party"]:
                run(f"{phase}_{name}", ["cargo", "+1.100.0", "test", "--offline", "--locked", "--quiet", "-p", "oteryn-protocol-oteryn", "--lib", f"{name}::tests::"], fails)
    finally:
        write(candidate)
    run("format_changed", ["rustfmt", "+1.100.0", "--config-path", "rustfmt.toml", *ALL_PATHS])
    run("format_changed_check", ["rustfmt", "+1.100.0", "--config-path", "rustfmt.toml", "--check", *ALL_PATHS])
    report["candidate_sha256"] = {p: hashlib.sha256((root / p).read_bytes()).hexdigest() for p in ALL_PATHS}
    (evidence / "formatted-candidate-files.json").write_text(json.dumps({p: (root / p).read_text() for p in ALL_PATHS}))
    packages = ["-p", "oteryn-identity", "-p", "oteryn-protocol-oteryn", "-p", "oteryn-gameplay-foundation"]
    run("candidate_unit", ["cargo", "+1.100.0", "test", "--offline", "--locked", "--quiet", *packages, "--lib"])
    run("candidate_clippy", ["cargo", "+1.100.0", "clippy", "--offline", "--locked", "--quiet", *packages, "--all-targets", "--", "-D", "warnings"])
    run("candidate_diff_check", ["git", "diff", "--check"])


if __name__ == "__main__":
    main()
