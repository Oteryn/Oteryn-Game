#!/usr/bin/env python3
"""Run the real S3B verdict tail with controlled command outcomes; no topology/build.

This tests the shell caller's oracle. The substituted command is explicitly a
synthetic libtest-output producer, not a running Game/Platform qualification.
"""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import shlex
import subprocess
import sys
import tempfile


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: server_boundary_repros.py /path/to/checkout")
    repo = Path(sys.argv[1]).resolve()
    path = repo / "tools/qualification/wp5_s3b/run.sh"
    source = path.read_text()
    cleanup = source[source.index("cleanup() {\n"):source.index("\n# Ephemeral fresh-admission signing key")]
    tail_start = source.index('(\n  cd "$SEAM_TEST_CWD"\n')
    tail = source[tail_start:]
    assert tail.endswith('result="$PASS_RESULT"\n')
    assert '"${SEAM_TEST_COMMAND[@]}"' in tail
    cases = []
    for name, command_output, returncode, expected in [
        ("zero_tests", "running 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; 1 filtered out\n", 0, "SEAM_PASS"),
        ("one_test_control", "running 1 test\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 filtered out\n", 0, "SEAM_PASS"),
        ("failed_command_control", "test result: FAILED. 0 passed; 1 failed\n", 1, "COMPOSED_FAIL"),
    ]:
        with tempfile.TemporaryDirectory(prefix="server-boundary-oracle-") as directory:
            work = Path(directory)
            cleanup_work = work / "cleanup"
            cleanup_work.mkdir()
            command = work / "synthetic_libtest.py"
            command.write_text(f"import sys\nprint({command_output!r}, end='')\nsys.exit({returncode})\n")
            setup = "\n".join([
                "set -Eeuo pipefail",
                "result=COMPOSED_FAIL",
                "PASS_RESULT=SEAM_PASS",
                "cleaned=true",
                f"WORK={shlex.quote(str(cleanup_work))}",
                f"SEAM_TEST_CWD={shlex.quote(str(work))}",
                f"SEAM_TEST_COMMAND=({shlex.quote(sys.executable)} {shlex.quote(str(command))})",
                "FRESH_SEED_HEX=synthetic-unused-value",
                "RECOVERY_SEED_HEX=synthetic-unused-value",
                "evidence() { printf 'S3B_EVIDENCE %s\\n' \"$*\"; }",
            ])
            process = subprocess.run(["bash"], input=setup + "\n" + cleanup + "\n" + tail,
                                     text=True, capture_output=True, timeout=5)
            assert process.returncode == returncode, process
            assert f"S3B_RESULT={expected}\n" in process.stdout, process
            cases.append({
                "case": name,
                "synthetic_command_rc": returncode,
                "actual_shell_rc": process.returncode,
                "verdict": expected,
                "stdout": process.stdout,
                "stderr": process.stderr,
                "oracle_gap": name == "zero_tests" and expected == "SEAM_PASS",
            })
    result = {
        "scope": "actual shell cleanup and execution/verdict tail, synthetic test command only",
        "source": str(path.relative_to(repo)),
        "source_sha256": hashlib.sha256(source.encode()).hexdigest(),
        "tail_first_line": source[:tail_start].count("\n") + 1,
        "topology_started": False,
        "rust_tests_executed": False,
        "cases": cases,
    }
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
