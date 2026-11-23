"""Run a declared batch of test-oracle mutations in the validation-only checkout.

The JSON payload names complete candidate test-bearing source files and exact
temporary mutations. Original source is restored before final qualification.
Unmodified protocol Rust files may be converted from CRLF only after proving
that their canonical bytes equal the exact baseline. No git configuration,
publication, unrelated file or live service is changed.
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
PROTECTED = "crates/input-platform/tests/native_winit.rs"


def main():
    parser = argparse.ArgumentParser()
    for name in ["worktree", "evidence", "payload", "target"]:
        parser.add_argument(f"--{name}", type=Path, required=True)
    args = parser.parse_args()
    root, evidence = args.worktree.resolve(), args.evidence.resolve()
    assert subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root).decode().strip() == BASE
    payload = json.loads(args.payload.read_text(encoding="utf-8"))
    candidates, cases = payload["candidate"], payload["cases"]
    retained = payload.get("retained", {})
    for path, digest in retained.items():
        assert hashlib.sha256((root / path).read_bytes()).hexdigest() == digest, path
    protected_hash = hashlib.sha256((root / PROTECTED).read_bytes()).hexdigest()
    paths = set(candidates) | {case["path"] for case in cases}
    baseline = {p: subprocess.check_output(["git", "show", f"{BASE}:{p}"], cwd=root).decode("utf-8") for p in paths}
    for path in paths:
        assert (root / path).read_bytes().replace(b"\r\n", b"\n") == baseline[path].encode("utf-8"), path
    evidence.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, CARGO_TARGET_DIR=str(args.target), CARGO_BUILD_JOBS="4")
    report = {"baseline": BASE, "rust_toolchain": "1.100.0", "target": "x86_64-pc-windows-msvc", "protected_input_sha256": protected_hash, "retained_candidate_sha256": retained, "runs": []}

    def write(path, text):
        (root / path).write_bytes(text.encode("utf-8"))

    def record():
        (evidence / "results.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8", newline="\n")

    def run(label, command, fails=False, expect_tests=False):
        started = time.monotonic()
        result = subprocess.run(command, cwd=root, env=env, text=True, encoding="utf-8", errors="replace", stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=1200)
        (evidence / f"{label}.log").write_text(result.stdout, encoding="utf-8", newline="\n")
        summaries = re.findall(r"test result: .*", result.stdout)
        entry = {"label": label, "command": command, "returncode": result.returncode, "seconds": round(time.monotonic() - started, 3), "summaries": summaries}
        report["runs"].append(entry)
        record()
        print(json.dumps(entry), flush=True)
        assert result.returncode == (101 if fails else 0), result.stdout[-4000:]
        if fails:
            assert "test result: FAILED" in result.stdout
        elif expect_tests:
            assert any(int(n) > 0 for n in re.findall(r"test result: ok\. (\d+) passed", result.stdout)), summaries

    final = dict(baseline, **candidates)
    try:
        for phase, files, fails in [("baseline", baseline, False), ("candidate", final, True)]:
            for case in cases:
                mutated = dict(files)
                text = mutated[case["path"]]
                assert text.count(case["old"]) == 1, case["label"]
                mutated[case["path"]] = text.replace(case["old"], case["new"], 1)
                for path, text in mutated.items():
                    write(path, text)
                run(f'{phase}_mutant_{case["label"]}', ["cargo", "+1.100.0", "test", "--locked", "--quiet", "-p", "oteryn-protocol-oteryn", "--lib", case["test"]], fails=fails, expect_tests=True)
    finally:
        for path, text in final.items():
            write(path, text)
    canonicalized = []
    tracked = subprocess.check_output(["git", "ls-files", "crates/protocol-oteryn/*.rs"], cwd=root).decode().splitlines()
    for path in tracked:
        if path in candidates:
            continue
        if path in retained:
            assert hashlib.sha256((root / path).read_bytes()).hexdigest() == retained[path], path
            continue
        canonical = subprocess.check_output(["git", "show", f"{BASE}:{path}"], cwd=root)
        current = (root / path).read_bytes()
        assert current.replace(b"\r\n", b"\n") == canonical, path
        if current != canonical:
            (root / path).write_bytes(canonical)
            canonicalized.append(path)
    report["canonicalized_unchanged_paths"] = canonicalized
    run("format_changed", ["rustfmt", "+1.100.0", "--config-path", "rustfmt.toml", *candidates])
    report["candidate_sha256"] = {p: hashlib.sha256((root / p).read_bytes()).hexdigest() for p in candidates}
    report["restored_production_sha256"] = {p: hashlib.sha256((root / p).read_bytes()).hexdigest() for p in paths - set(candidates)}
    (evidence / "formatted-candidate-files.json").write_text(json.dumps({p: (root / p).read_text(encoding="utf-8") for p in candidates}), encoding="utf-8", newline="\n")
    run("candidate_unit", ["cargo", "+1.100.0", "test", "--locked", "--quiet", "-p", "oteryn-protocol-oteryn", "--lib"], expect_tests=True)
    run("candidate_clippy", ["cargo", "+1.100.0", "clippy", "--locked", "--quiet", "-p", "oteryn-protocol-oteryn", "--all-targets", "--", "-D", "warnings"])
    run("format_package_check", ["cargo", "+1.100.0", "fmt", "--check", "-p", "oteryn-protocol-oteryn"])
    run("candidate_diff_check", ["git", "diff", "--check", "--", "crates/protocol-oteryn"])
    assert hashlib.sha256((root / PROTECTED).read_bytes()).hexdigest() == protected_hash
    for path, digest in retained.items():
        assert hashlib.sha256((root / path).read_bytes()).hexdigest() == digest, path
    report["protected_input_unchanged"] = True
    report["completed"] = True
    record()


if __name__ == "__main__":
    main()
