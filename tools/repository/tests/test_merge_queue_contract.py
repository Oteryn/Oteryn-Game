"""Regression checks for the native Merge Queue aggregate gate contract."""

from __future__ import annotations

import subprocess
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
WORKFLOW = ROOT / ".github/workflows/merge-gate.yml"
VALIDATOR = ROOT / "tools/repository/validate_repository_policy.py"
OBSOLETE_AUDIT = ROOT / ".github/workflows/merge-authority-audit.yml"


class MergeQueueContractTests(unittest.TestCase):
    def test_aggregate_gate_handles_pr_and_merge_group_candidates(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")

        self.assertIn("  pull_request:\n", workflow)
        self.assertIn("  merge_group:\n    types:\n      - checks_requested\n", workflow)
        self.assertIn("EVENT_NAME: ${{ github.event_name }}", workflow)
        self.assertIn("EVENT_CANDIDATE_SHA: ${{ github.sha }}", workflow)
        self.assertIn("if event_name == 'merge_group':", workflow)
        self.assertIn("target_sha = candidate_sha", workflow)
        self.assertIn("name: game-gate", workflow)
        self.assertIn("needs: validate", workflow)

    def test_pr_only_metadata_jobs_are_not_required_for_merge_group(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")

        self.assertIn("if: github.event_name == 'pull_request'", workflow)
        self.assertIn("if: github.event_name == 'merge_group'", workflow)
        self.assertIn("INTEGRATION_MODE: ${{ needs.scope.outputs.integration_mode }}", workflow)

    def test_obsolete_merge_authority_audit_is_removed(self) -> None:
        self.assertFalse(OBSOLETE_AUDIT.exists())
        self.assertNotIn("merge-authority-audit", WORKFLOW.read_text(encoding="utf-8"))
        self.assertNotIn("merge-authority-audit", VALIDATOR.read_text(encoding="utf-8"))

    def test_policy_validator_accepts_the_native_contract(self) -> None:
        result = subprocess.run(
            [sys.executable, str(VALIDATOR)],
            cwd=ROOT,
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
