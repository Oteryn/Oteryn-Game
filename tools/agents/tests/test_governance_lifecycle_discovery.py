#!/usr/bin/env python3
"""Prove that lifecycle discovery runs its bodies and rejects a failing assertion."""

from __future__ import annotations

import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
TESTS = Path("tools/agents/tests")
LIFECYCLE_TEST = TESTS / "test_validate_governance_lifecycle.py"
FIXTURE_FILES = (
    Path("tools/agents/validate_governance.py"),
    Path("tools/agents/validate_governance_core.py"),
    LIFECYCLE_TEST,
)
EXPECTED_CASES = (
    "test_prompt_registry_covers_every_prompt_and_retires_with_successor",
    "test_handover_registry_requires_non_authority_expiry_and_supersession",
    "test_active_task_packets_require_github_authority_and_nonterminal_status",
)
CANARY = "F02_DISCOVERY_NEGATIVE_CANARY"


class GovernanceLifecycleDiscoveryTests(unittest.TestCase):
    def run_discovery(self, *, inject_failure: bool) -> subprocess.CompletedProcess[str]:
        # Never mutate the checkout. Run the same files and Python interpreter
        # in a disposable tree, keeping the runtime wrapper/core boundary real.
        with tempfile.TemporaryDirectory() as directory:
            snapshot = Path(directory)
            for relative in FIXTURE_FILES:
                destination = snapshot / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(ROOT / relative, destination)
            if inject_failure:
                test_path = snapshot / LIFECYCLE_TEST
                text = test_path.read_text(encoding="utf-8")
                needle = '        self.assertIn("retired prompt A must name superseded_by", errors)'
                self.assertEqual(text.count(needle), 1, "negative canary anchor drifted")
                test_path.write_text(
                    text.replace(needle, f'        self.fail("{CANARY}")'),
                    encoding="utf-8",
                )
            return subprocess.run(
                [
                    sys.executable, "-m", "unittest", "discover",
                    "-s", str(TESTS), "-p", LIFECYCLE_TEST.name, "-v",
                ],
                cwd=snapshot,
                text=True,
                capture_output=True,
                timeout=30,
                check=False,
            )

    def assert_cases_executed(self, output: str) -> None:
        self.assertIn("Ran 3 tests", output)
        for name in EXPECTED_CASES:
            self.assertRegex(output, rf"(?m)^{name} \([^\n]+\) \.\.\. (ok|FAIL)$")

    def test_discovery_executes_all_lifecycle_bodies(self) -> None:
        result = self.run_discovery(inject_failure=False)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assert_cases_executed(result.stderr)
        self.assertIn("\nOK\n", result.stderr)

    def test_discovery_rejects_an_injected_lifecycle_assertion(self) -> None:
        result = self.run_discovery(inject_failure=True)
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assert_cases_executed(result.stderr)
        self.assertIn(f"FAIL: {EXPECTED_CASES[0]}", result.stderr)
        self.assertIn(CANARY, result.stderr)
        self.assertIn("FAILED (failures=1)", result.stderr)
        self.assertNotIn("ERROR:", result.stderr)


if __name__ == "__main__":
    unittest.main()
