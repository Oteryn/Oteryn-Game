"""Reproduce the bounded oracle comparison in a disposable, exact-baseline worktree.

Candidate files are supplied as a JSON path-to-text map. Only the five named files
are touched; no commit, push, dependency substitution or production deployment occurs.
The registry probe corrupts the aggregate passed to the test helper, not the builder.
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
IDENTITY = "crates/identity/src/lib.rs"
DAILY = "crates/protocol-oteryn/src/daily_boost.rs"
GUILD = "crates/protocol-oteryn/src/guild_war.rs"
REGISTRY = "crates/protocol-oteryn/src/contract_registries.rs"
SCHEDULE = "crates/gameplay/foundation/src/scheduled_events.rs"
PATHS = [IDENTITY, DAILY, GUILD, REGISTRY, SCHEDULE]
CASES = [
    ("pkce", "oteryn-identity", "pkce_"),
    ("daily", "oteryn-protocol-oteryn", "daily_boost::tests::"),
    ("guild", "oteryn-protocol-oteryn", "guild_war::tests::round_trip"),
    ("registry", "oteryn-protocol-oteryn", "contract_registries::tests::"),
]


def replace_once(text, old, new):
    assert text.count(old) == 1, old
    return text.replace(old, new, 1)


def mutate(files):
    files = dict(files)
    old = "let challenge = base64_url_no_pad(&Sha256::digest(verifier.as_bytes()));"
    files[IDENTITY] = replace_once(files[IDENTITY], old, old + '\n        let challenge = "A".repeat(challenge.len());')
    files[DAILY] = files[DAILY].replace(".to_be_bytes()", ".to_le_bytes()").replace("u64::from_be_bytes", "u64::from_le_bytes")
    files[GUILD] = replace_once(files[GUILD], "    if bytes.len() > MAX_GUILD_WAR_REQUEST_BYTES {\n        return Err(CyclopediaWireError::LimitExceeded);\n    }\n", "")
    old = '        let aggregate: Value = serde_json::from_str(aggregate).expect("aggregate JSON");'
    new = old.replace("let aggregate:", "let mut aggregate:") + '''
        for key in collections {
            let entry = aggregate[*key].as_array_mut().expect("collection")
                .first_mut().expect("nonempty collection").as_object_mut().expect("entry");
            entry.insert("audit_mutant_field".to_owned(), Value::Bool(true));
        }'''
    files[REGISTRY] = replace_once(files[REGISTRY], old, new)
    return files


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--worktree", type=Path, required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--candidate-json", type=Path, required=True)
    parser.add_argument("--target", type=Path, required=True)
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

    def run(label, command, expected_failure=False):
        started = time.monotonic()
        result = subprocess.run(command, cwd=root, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=900)
        (evidence / f"{label}.log").write_text(result.stdout)
        summaries = re.findall(r"test result: .*", result.stdout)
        record = {"label": label, "command": command, "returncode": result.returncode, "seconds": round(time.monotonic() - started, 3), "summaries": summaries}
        report["runs"].append(record)
        (evidence / "results.json").write_text(json.dumps(report, indent=2) + "\n")
        print(json.dumps(record), flush=True)
        if expected_failure:
            assert result.returncode == 101 and "test result: FAILED" in result.stdout, result.stdout[-4000:]
        else:
            assert result.returncode == 0, result.stdout[-4000:]

    try:
        for phase, files, failed in [("baseline_mutant", baseline, False), ("candidate_mutant", candidate, True)]:
            write(mutate(files))
            for name, package, test_filter in CASES:
                run(f"{phase}_{name}", ["cargo", "+1.100.0", "test", "--offline", "--locked", "--quiet", "-p", package, "--lib", test_filter], failed)
    finally:
        write(candidate)
    run("format_changed", ["rustfmt", "+1.100.0", "--config-path", "rustfmt.toml", *PATHS])
    report["candidate_sha256"] = {p: hashlib.sha256((root / p).read_bytes()).hexdigest() for p in PATHS}
    (evidence / "formatted-candidate-files.json").write_text(json.dumps({p: (root / p).read_text() for p in PATHS}))
    run("format_all_check", ["cargo", "+1.100.0", "fmt", "--all", "--check"])
    packages = ["-p", "oteryn-identity", "-p", "oteryn-protocol-oteryn", "-p", "oteryn-gameplay-foundation"]
    run("candidate_unit", ["cargo", "+1.100.0", "test", "--offline", "--locked", "--quiet", *packages, "--lib"])
    run("candidate_clippy", ["cargo", "+1.100.0", "clippy", "--offline", "--locked", "--quiet", *packages, "--all-targets", "--", "-D", "warnings"])
    run("candidate_diff_check", ["git", "diff", "--check"])


if __name__ == "__main__":
    main()
