#!/usr/bin/env python3
"""Every tools/repository test runs in the merge gate's governance job.

A guard nobody runs guards nothing: test_heavy_gate.py stayed red on main
because no workflow executed it.
"""
from pathlib import Path
import os
import re
import subprocess
import sys
import tempfile
import textwrap
import unittest

from validate_repository_policy_core import indented_yaml_mapping_block

ROOT = Path(__file__).resolve().parents[2]
LOOP = "          for test in tools/repository/test_*.py; do\n"


class RepositoryTestsRunTests(unittest.TestCase):
    def setUp(self):
        text = (ROOT / ".github/workflows/merge-gate.yml").read_text()
        self.job = indented_yaml_mapping_block(text, "governance", 2)
        self.assertIsNotNone(self.job)
        self.step = next(step for step in re.split(r"(?m)^      - ", self.job) if LOOP in step)

    def run_loop(self, names, failing=""):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            tests = root / "tools/repository"
            tests.mkdir(parents=True)
            for name in names:
                (tests / name).touch()
            trace = root / "calls"
            python = root / "python"
            python.write_text(f"#!{sys.executable}\n" + textwrap.dedent('''\
                import os, sys
                from pathlib import Path
                name = Path(sys.argv[1]).name
                with Path(os.environ["OTERYN_LOOP_TRACE"]).open("a") as output:
                    output.write(name + "\\n")
                sys.exit(42 if name == os.environ["OTERYN_LOOP_FAIL"] else 0)
            '''))
            python.chmod(0o700)
            script = textwrap.dedent(self.step.split("        run: |\n", 1)[1])
            result = subprocess.run(
                ["bash", "-c", script], cwd=root,
                env={**os.environ, "PATH": str(root) + os.pathsep + os.defpath,
                     "OTERYN_LOOP_TRACE": str(trace), "OTERYN_LOOP_FAIL": failing},
                capture_output=True, text=True, timeout=20, check=False,
            )
            return result, trace.read_text().splitlines() if trace.exists() else []

    def test_governance_runs_every_repository_test(self):
        self.assertIn(LOOP, self.job)
        self.assertIn('            python "$test"\n', self.job)
        loop = self.job.split(LOOP, 1)[1].split("          done\n", 1)[0]
        skipped = re.findall(r"tools/repository/(test_[a-z0-9_]+\.py)", loop)
        for name in skipped:
            with self.subTest(skipped=name):
                self.assertIn(f"          python tools/repository/{name}\n", self.job,
                              "a test skipped by the loop must run explicitly in the same job")
        tests = sorted(p.name for p in (ROOT / "tools/repository").glob("test_*.py"))
        self.assertIn(Path(__file__).name, tests)
        result, calls = self.run_loop(tests)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertCountEqual(calls, [name for name in tests if name not in skipped])

    def test_loop_steps_share_the_job_result_cache_condition(self):
        self.assertIn("        if: steps.jr_hit.outputs.hit != 'true'\n", self.step + "\n")
        self.assertIn("          set -euo pipefail\n", self.step + "\n")
        result, calls = self.run_loop(["test_000_failure.py", "test_999_later.py"], "test_000_failure.py")
        self.assertEqual(result.returncode, 42, result.stderr)
        self.assertEqual(calls, ["test_000_failure.py"])


if __name__ == "__main__":
    unittest.main()
