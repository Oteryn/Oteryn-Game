#!/usr/bin/env python3
"""Lifecycle regression tests for the existing Game governance validator."""

from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path

VALIDATOR_PATH = Path(__file__).resolve().parents[1] / "validate_governance.py"
SPEC = importlib.util.spec_from_file_location("validate_governance", VALIDATOR_PATH)
assert SPEC is not None and SPEC.loader is not None
validator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validator)


class GovernanceLifecycleTests(unittest.TestCase):
    def setUp(self) -> None:
        self.original_root = validator.ROOT
        self.tempdir = tempfile.TemporaryDirectory()
        validator.ROOT = Path(self.tempdir.name)

    def tearDown(self) -> None:
        validator.ROOT = self.original_root
        self.tempdir.cleanup()

    def write(self, relative: str, content: str = "# fixture\n") -> None:
        path = validator.ROOT / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")

    def test_prompt_registry_covers_every_prompt_and_retires_with_successor(self) -> None:
        self.write("docs/agents/prompts/A.md")
        self.write("docs/agents/prompts/B.md")
        registry = {
            "prompts": [
                {
                    "prompt_id": "A",
                    "path": "docs/agents/prompts/A.md",
                    "version": "1.0",
                    "status": "retired",
                    "owner": "governance",
                    "scope": "fixture",
                    "reusable": False,
                    "superseded_by": None,
                    "supersession_rule": "explicit replacement only",
                }
            ]
        }
        errors: list[str] = []
        validator.validate_prompt_lifecycle(registry, errors)
        self.assertIn("prompt lifecycle registry missing paths: docs/agents/prompts/B.md", errors)
        self.assertIn("retired prompt A must name superseded_by", errors)

    def test_handover_registry_requires_non_authority_expiry_and_supersession(self) -> None:
        self.write("docs/agents/evidence/OTV2-test-handoff.md")
        registry = {
            "handovers": [
                {
                    "handover_id": "test-handoff",
                    "path": "docs/agents/evidence/OTV2-test-handoff.md",
                    "status": "historical",
                    "authoritative": True,
                    "expiry_rule": "",
                    "superseded_by": [],
                }
            ]
        }
        errors: list[str] = []
        validator.validate_handover_lifecycle(registry, errors)
        self.assertIn("handover test-handoff must be explicitly non-authoritative", errors)
        self.assertIn("handover test-handoff must define expiry_rule", errors)
        self.assertIn("handover test-handoff must define superseded_by", errors)

    def test_active_task_packets_require_github_authority_and_nonterminal_status(self) -> None:
        self.write(
            "docs/agents/tasks/active/OTV2-no-authority.md",
            "# task\n```yaml\nstatus: implementing\npr: null\n```\n",
        )
        self.write(
            "docs/agents/tasks/active/OTV2-terminal.md",
            "# task\n```yaml\nstatus: completed\nissue: 123\n```\n",
        )
        errors: list[str] = []
        validator.validate_active_task_packets(errors)
        self.assertIn(
            "active task packet docs/agents/tasks/active/OTV2-no-authority.md must name a positive issue or pr",
            errors,
        )
        self.assertIn(
            "active task packet docs/agents/tasks/active/OTV2-terminal.md has terminal status completed",
            errors,
        )


if __name__ == "__main__":
    unittest.main()
