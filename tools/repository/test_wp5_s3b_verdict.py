#!/usr/bin/env python3
"""Exercise the S3B caller verdict with controlled libtest output, without topology."""

from pathlib import Path
import shlex
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "tools/qualification/wp5_s3b/run.sh"
PASS_LINE = "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n"


class SeamVerdictTests(unittest.TestCase):
    def run_tail(self, output, returncode=0, pass_result="SEAM_PASS"):
        source = SOURCE.read_text()
        cleanup = source[source.index("cleanup() {\n"):
                         source.index("\n# Ephemeral fresh-admission signing key")]
        tail = source[source.index('(\n  cd "$SEAM_TEST_CWD"\n'):]
        with tempfile.TemporaryDirectory(prefix="s3b-verdict-") as directory:
            work = Path(directory)
            cleanup_work = work / "cleanup"
            cleanup_work.mkdir()
            command = work / "synthetic_libtest.py"
            command.write_text(f"import sys\nprint({output!r}, end='')\nsys.exit({returncode})\n")
            setup = "\n".join([
                "set -Eeuo pipefail",
                "result=COMPOSED_FAIL",
                f"PASS_RESULT={shlex.quote(pass_result)}",
                "cleaned=true",
                f"WORK={shlex.quote(str(cleanup_work))}",
                f"SEAM_TEST_CWD={shlex.quote(str(work))}",
                f"SEAM_TEST_COMMAND=({shlex.quote(sys.executable)} {shlex.quote(str(command))})",
                "FRESH_SEED_HEX=synthetic-unused-value",
                "RECOVERY_SEED_HEX=synthetic-unused-value",
                "evidence() { printf 'S3B_EVIDENCE %s\\n' \"$*\"; }",
            ])
            return subprocess.run(["bash"], input=setup + "\n" + cleanup + "\n" + tail,
                                  text=True, capture_output=True, timeout=5)

    def test_zero_tests_cannot_qualify_any_variant(self):
        output = "running 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s\n"
        for variant in ("COMPOSED_PASS", "SEAM_PASS", "SPELL_SEAM_PASS"):
            with self.subTest(variant=variant):
                result = self.run_tail(output, pass_result=variant)
                self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
                self.assertIn("S3B_RESULT=COMPOSED_FAIL\n", result.stdout)
                self.assertIn("did not execute exactly one passing test", result.stderr)

    def test_exactly_one_successful_test_preserves_variant_and_output(self):
        output = "running 1 test\nS3B_EVIDENCE fixture=synthetic\n" + PASS_LINE
        for variant in ("COMPOSED_PASS", "SEAM_PASS", "SPELL_SEAM_PASS"):
            with self.subTest(variant=variant):
                result = self.run_tail(output, pass_result=variant)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertIn(output, result.stdout)
                self.assertIn(f"S3B_RESULT={variant}\n", result.stdout)

    def test_missing_wrong_or_ambiguous_result_is_rejected(self):
        for output in (
            "", "running 1 test\n", "prefix " + PASS_LINE,
            PASS_LINE.replace("1 passed", "2 passed"),
            PASS_LINE.replace("0 ignored", "1 ignored"),
            PASS_LINE.replace("0 failed", "1 failed"),
            PASS_LINE.replace("0 measured", "1 measured"),
            PASS_LINE.split("0 measured", 1)[0] + "\n",
            PASS_LINE.rstrip("\n") + " trailing\n",
            PASS_LINE + PASS_LINE,
            PASS_LINE + PASS_LINE.replace("1 passed", "0 passed"),
        ):
            with self.subTest(output=output):
                result = self.run_tail(output)
                self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
                self.assertIn("S3B_RESULT=COMPOSED_FAIL\n", result.stdout)

    def test_command_failure_cannot_be_hidden_by_a_successful_summary(self):
        for code in (1, 7):
            with self.subTest(returncode=code):
                result = self.run_tail(PASS_LINE, returncode=code)
                self.assertEqual(result.returncode, code, result.stdout + result.stderr)
                self.assertIn("S3B_RESULT=COMPOSED_FAIL\n", result.stdout)
                self.assertNotIn("S3B_RESULT=SEAM_PASS\n", result.stdout)


if __name__ == "__main__":
    unittest.main()
