#!/usr/bin/env python3
"""Exercise each actual S3B caller tail with controlled child output, without topology."""

import os
from pathlib import Path
import shlex
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "tools/qualification/wp5_s3b/run.sh"
PASS_LINE = "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n"
MODES = {
    "s3b": (
        "real_owners_compose_fresh_admission_and_fence_replacement",
        "S3B_EVIDENCE composition=complete", "COMPOSED_PASS",
    ),
    "seam": (
        "qualification::server_seam_real_owners_over_tcp_tls",
        "SEAM_EVIDENCE shutdown=drained FORMAL_ADR0007_QA_TIER1_TIER2=NOT_EVALUATED",
        "SEAM_PASS",
    ),
    "spell-seam": (
        "qualification::server_seam_real_owners_over_tcp_tls",
        "SEAM_EVIDENCE spell_seam shutdown=drained scope=real_owner_tcp_tls gameplay_complete=false",
        "SPELL_SEAM_PASS",
    ),
}


class SeamVerdictTests(unittest.TestCase):
    def run_tail(self, mode, output, returncode=0, stderr=""):
        source = SOURCE.read_text()
        select = source[source.index('case "${WP5_QUALIFICATION:-s3b}" in'):source.index('\nesac') + 6]
        cleanup = source[source.index("cleanup() {\n"):
                         source.index("\n# Ephemeral fresh-admission signing key")]
        tail = source[source.index('(\n  cd "$SEAM_TEST_CWD"\n'):]
        with tempfile.TemporaryDirectory(prefix="s3b-verdict-") as directory:
            work = Path(directory)
            cleanup_work = work / "cleanup"
            cleanup_work.mkdir()
            command = work / "synthetic_libtest.py"
            command.write_text(
                f"import sys\nprint({output!r}, end='', flush=True)\n"
                f"print({stderr!r}, end='', file=sys.stderr, flush=True)\nsys.exit({returncode})\n"
            )
            setup = "\n".join([
                "set -Eeuo pipefail", select, "result=COMPOSED_FAIL", "cleaned=true",
                f"WORK={shlex.quote(str(cleanup_work))}",
                f"SEAM_TEST_CWD={shlex.quote(str(work))}",
                f"SEAM_TEST_COMMAND=({shlex.quote(sys.executable)} {shlex.quote(str(command))})",
                "FRESH_SEED_HEX=synthetic-unused-value",
                "RECOVERY_SEED_HEX=synthetic-unused-value",
                "evidence() { printf 'S3B_EVIDENCE %s\\n' \"$*\"; }",
            ])
            env = {**os.environ, "WP5_QUALIFICATION": mode}
            return subprocess.run(["bash"], input=setup + "\n" + cleanup + "\n" + tail,
                                  text=True, capture_output=True, timeout=5, env=env)

    def success_output(self, mode):
        name, marker, _ = MODES[mode]
        # Under --nocapture the first body print may follow libtest's name.
        return f"running 1 test\ntest {name} ... body-started\n{marker}\nok\n\n" + PASS_LINE

    def assert_refused(self, mode, output, returncode=1):
        result = self.run_tail(mode, output)
        self.assertEqual(result.returncode, returncode, result.stdout + result.stderr)
        self.assertIn("S3B_RESULT=COMPOSED_FAIL\n", result.stdout)
        self.assertNotIn(f"S3B_RESULT={MODES[mode][2]}\n", result.stdout)

    def test_zero_tests_cannot_qualify_any_variant(self):
        output = "running 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s\n"
        for mode in MODES:
            with self.subTest(mode=mode):
                self.assert_refused(mode, output)

    def test_exactly_one_successful_test_preserves_variant_and_output(self):
        for mode, (_, _, passed) in MODES.items():
            with self.subTest(mode=mode):
                output = self.success_output(mode)
                result = self.run_tail(mode, output)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertIn(output, result.stdout)
                self.assertIn(f"S3B_RESULT={passed}\n", result.stdout)

    def test_missing_wrong_or_ambiguous_result_is_rejected(self):
        summaries = (
            "", "prefix " + PASS_LINE,
            PASS_LINE.replace("1 passed", "2 passed"),
            PASS_LINE.replace("0 ignored", "1 ignored"),
            PASS_LINE.replace("0 failed", "1 failed"),
            PASS_LINE.replace("0 measured", "1 measured"),
            PASS_LINE.replace("1 passed; 0 failed; 0 ignored", "0 passed; 0 failed; 1 ignored"),
            PASS_LINE.split("0 measured", 1)[0] + "\n",
            PASS_LINE.rstrip("\n") + " trailing\n",
            PASS_LINE + PASS_LINE,
            PASS_LINE + PASS_LINE.replace("1 passed", "0 passed"),
        )
        for mode in MODES:
            for summary in summaries:
                with self.subTest(mode=mode, summary=summary):
                    self.assert_refused(mode, self.success_output(mode).replace(PASS_LINE, summary))
            for absent in ("", "running 1 test\n"):
                with self.subTest(mode=mode, absent=absent):
                    self.assert_refused(mode, absent)

    def test_unselected_case_or_missing_terminal_body_cannot_qualify(self):
        for mode, (name, marker, _) in MODES.items():
            good = self.success_output(mode)
            invalid = {
                "wrong-case": good.replace(name, "unrelated_case"),
                "name-only-in-body": good.replace(name, "unrelated_case") + f"reference to test {name} ... unexecuted\n",
                "missing-body-proof": good.replace(marker, "unrelated evidence"),
                "wrong-body-proof": good.replace(marker, MODES["seam" if mode != "seam" else "s3b"][1]),
            }
            for failure, output in invalid.items():
                with self.subTest(mode=mode, failure=failure):
                    self.assert_refused(mode, output)

    def test_command_failure_cannot_be_hidden_by_complete_output(self):
        for mode in MODES:
            for code in (1, 7):
                with self.subTest(mode=mode, returncode=code):
                    result = self.run_tail(mode, self.success_output(mode), returncode=code)
                    self.assertEqual(result.returncode, code, result.stdout + result.stderr)
                    self.assertIn("S3B_RESULT=COMPOSED_FAIL\n", result.stdout)
                    self.assertNotIn(f"S3B_RESULT={MODES[mode][2]}\n", result.stdout)

    def test_terminal_body_evidence_on_stderr_is_retained(self):
        for mode, (name, marker, passed) in MODES.items():
            with self.subTest(mode=mode):
                stdout = f"running 1 test\ntest {name} ... body-started\n"
                stderr = f"{marker}\nok\n\n" + PASS_LINE
                result = self.run_tail(mode, stdout, stderr=stderr)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertIn(marker, result.stdout)
                self.assertIn(f"S3B_RESULT={passed}\n", result.stdout)


if __name__ == "__main__":
    unittest.main()
