#!/usr/bin/env python3
"""Every tools/repository test runs in the merge gate's governance job.

A guard nobody runs guards nothing: test_heavy_gate.py stayed red on main
because no workflow executed it.
"""
from pathlib import Path
import re
import unittest

from validate_repository_policy_core import indented_yaml_mapping_block

ROOT = Path(__file__).resolve().parents[2]
LOOP = "          for test in tools/repository/test_*.py; do\n"


class RepositoryTestsRunTests(unittest.TestCase):
    def setUp(self):
        text = (ROOT / ".github/workflows/merge-gate.yml").read_text()
        self.job = indented_yaml_mapping_block(text, "governance", 2)
        self.assertIsNotNone(self.job)

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
        self.assertGreaterEqual(len(tests), 18)

    def test_loop_steps_share_the_job_result_cache_condition(self):
        step = self.job.split("      - name: Run every repository tool test\n", 1)[1].split("\n\n", 1)[0]
        self.assertIn("        if: steps.jr_hit.outputs.hit != 'true'\n", step + "\n")
        self.assertIn("          set -euo pipefail\n", step + "\n")


if __name__ == "__main__":
    unittest.main()
