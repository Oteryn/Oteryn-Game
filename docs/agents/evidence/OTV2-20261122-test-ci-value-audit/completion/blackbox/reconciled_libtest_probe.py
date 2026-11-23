"""Qualify the actual merged S3B caller with Rust 1.100.0 libtest; no game topology."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shlex
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--repo", type=Path, required=True)
parser.add_argument("--output", type=Path, required=True)
args = parser.parse_args()
source = (args.repo / "tools/qualification/wp5_s3b/run.sh").read_text()
select = source[source.index('case "${WP5_QUALIFICATION:-s3b}" in'):source.index('\nesac') + 6]
cleanup = source[source.index("cleanup() {\n"):source.index("\n# Ephemeral fresh-admission signing key")]
tail = source[source.index('(\n  cd "$SEAM_TEST_CWD"\n'):]
rust = r"""
fn body() {
    println!("fixture body started");
    assert_ne!(std::env::var("PROBE_TERMINAL").as_deref(), Ok("failure"), "intentional assertion failure");
    if std::env::var("PROBE_TERMINAL").as_deref() == Ok("missing") { return; }
    let marker = match std::env::var("WP5_QUALIFICATION").as_deref() {
        Ok("s3b") => "S3B_EVIDENCE composition=complete",
        Ok("seam") => "SEAM_EVIDENCE shutdown=drained FORMAL_ADR0007_QA_TIER1_TIER2=NOT_EVALUATED",
        Ok("spell-seam") => "SEAM_EVIDENCE spell_seam shutdown=drained scope=real_owner_tcp_tls gameplay_complete=false",
        _ => panic!("unknown probe mode"),
    };
    if std::env::var("PROBE_TERMINAL").as_deref() == Ok("stderr") { eprintln!("{marker}"); }
    else { println!("{marker}"); }
}
#[test] #[ignore]
fn real_owners_compose_fresh_admission_and_fence_replacement() { body(); }
mod qualification {
    #[test] #[ignore]
    fn server_seam_real_owners_over_tcp_tls() { super::body(); }
}
#[test] #[ignore]
fn unrelated_case() { body(); }
"""
modes = {
    "s3b": ("real_owners_compose_fresh_admission_and_fence_replacement", "COMPOSED_PASS"),
    "seam": ("qualification::server_seam_real_owners_over_tcp_tls", "SEAM_PASS"),
    "spell-seam": ("qualification::server_seam_real_owners_over_tcp_tls", "SPELL_SEAM_PASS"),
}
record = {"scope": "Actual tiny Rust libtest, actual mode selection and actual S3B cleanup/verdict tail; no Game/PG/Platform run",
          "source_sha256": hashlib.sha256(source.encode()).hexdigest(), "fixture": rust, "cases": []}
with tempfile.TemporaryDirectory(prefix="oteryn-s3b-reconciled-") as temporary:
    work = Path(temporary)
    fixture = work / "fixture.rs"
    fixture.write_text(rust)
    binary = work / "libtest-probe"
    compiler = ["rustc", "+1.100.0"]
    record["rustc"] = subprocess.check_output([*compiler, "--version"], text=True).strip()
    built = subprocess.run([*compiler, "--edition=2024", "--test", str(fixture), "-o", str(binary)], text=True, capture_output=True, timeout=60)
    record["compile_exit_code"] = built.returncode
    if built.returncode:
        raise RuntimeError(built.stderr)
    for mode, (name, passed) in modes.items():
        for case, selected, ignored, terminal, expected in (
            ("success", name, True, "stdout", 0),
            ("stderr_terminal", name, True, "stderr", 0),
            ("no_exact_match", "missing_name", True, "stdout", 1),
            ("selected_but_ignored", name, False, "stdout", 1),
            ("failed_body", name, True, "failure", 101),
            ("missing_terminal", name, True, "missing", 1),
            ("unselected_case", "unrelated_case", True, "stdout", 1),
        ):
            cleanup_dir = work / (mode + "-" + case)
            cleanup_dir.mkdir()
            command = [str(binary), selected, "--exact", "--nocapture", "--color", "never"]
            if ignored:
                command.append("--ignored")
            setup = "\n".join([
                "set -Eeuo pipefail", select, "result=COMPOSED_FAIL", "cleaned=true",
                "WORK=" + shlex.quote(str(cleanup_dir)), "SEAM_TEST_CWD=" + shlex.quote(str(work)),
                "SEAM_TEST_COMMAND=(" + " ".join(map(shlex.quote, command)) + ")",
                "FRESH_SEED_HEX=synthetic-unused", "RECOVERY_SEED_HEX=synthetic-unused",
                'evidence() { printf "S3B_EVIDENCE %s\\n" "$*"; }',
            ])
            result = subprocess.run(["bash"], input=setup + "\n" + cleanup + "\n" + tail, text=True,
                                    capture_output=True, timeout=10,
                                    env={**os.environ, "WP5_QUALIFICATION": mode, "PROBE_TERMINAL": terminal})
            verdict = passed if expected == 0 else "COMPOSED_FAIL"
            good = result.returncode == expected and f"S3B_RESULT={verdict}\n" in result.stdout
            record["cases"].append({"mode": mode, "case": case, "selected": selected,
                                    "terminal": terminal, "expected_exit_code": expected,
                                    "exit_code": result.returncode, "matched_expected": good,
                                    "stdout": result.stdout, "stderr": result.stderr})
record["all_expected_outcomes"] = all(case["matched_expected"] for case in record["cases"])
args.output.parent.mkdir(parents=True, exist_ok=True)
args.output.write_text(json.dumps(record, indent=2) + "\n")
print(json.dumps({"rustc": record["rustc"], "cases": len(record["cases"]),
                  "all_expected_outcomes": record["all_expected_outcomes"]}))
raise SystemExit(0 if record["all_expected_outcomes"] else 1)
