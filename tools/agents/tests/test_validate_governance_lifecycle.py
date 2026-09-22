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
        self.write("docs/agents/prompts/retired/A.md")
        self.write("docs/agents/prompts/B.md")
        registry = {
            "prompts": [
                {
                    "prompt_id": "A",
                    "path": "docs/agents/prompts/retired/A.md",
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


    def test_program_registry_covers_archived_programmes_with_terminal_evidence(self) -> None:
        self.write("docs/agents/programs/archive/HISTORICAL.md")
        registry = {
            "programs": [
                {
                    "program_id": "historical",
                    "path": "docs/agents/programs/archive/HISTORICAL.md",
                    "status": "historical",
                    "authoritative": False,
                    "terminal_evidence": [],
                    "superseded_by": [],
                }
            ]
        }
        errors: list[str] = []
        validator.validate_program_lifecycle(registry, errors)
        self.assertIn("program historical must define terminal_evidence", errors)
        self.assertIn("program historical must define superseded_by", errors)

    def test_active_task_packets_reject_unknown_lifecycle_and_oversize_context(self) -> None:
        self.write(
            "docs/agents/tasks/active/OTV2-bad-status.md",
            "# task\n```yaml\nmode: IMPLEMENT\nstatus: review\nissue: 123\n```\n" + ("x" * 200),
        )
        errors: list[str] = []
        validator.validate_active_task_packets(
            errors,
            task_statuses=["investigating", "implementing", "validating", "ready", "waiting", "blocked", "completed"],
            task_modes=["IMPLEMENT"],
            limits={"max_characters": 100, "max_lines": 20},
        )
        self.assertIn(
            "active task packet docs/agents/tasks/active/OTV2-bad-status.md has unsupported status review",
            errors,
        )
        self.assertTrue(
            any("OTV2-bad-status.md exceeded bounded current-state size" in error for error in errors)
        )

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


    def test_active_task_packets_require_pr_binding_when_validating_or_ready(self) -> None:
        self.write(
            "docs/agents/tasks/active/OTV2-validating-without-pr.md",
            "# task\n```yaml\nmode: IMPLEMENT\nstatus: validating\nissue: 123\npr: null\n```\n",
        )
        self.write(
            "docs/agents/tasks/active/OTV2-waiting-without-pr.md",
            "# task\n```yaml\nmode: IMPLEMENT\nstatus: waiting\nissue: 124\npr: null\n```\n",
        )
        errors: list[str] = []
        validator.validate_active_task_packets(errors)
        self.assertIn(
            "active task packet docs/agents/tasks/active/OTV2-validating-without-pr.md "
            "with status validating must bind a positive canonical pr",
            errors,
        )
        self.assertFalse(
            any(
                "OTV2-waiting-without-pr.md" in error and "must bind a positive canonical pr" in error
                for error in errors
            )
        )

    def test_active_task_live_state_rejects_terminal_canonical_authority(self) -> None:
        self.write(
            "docs/agents/tasks/active/OTV2-merged-pr.md",
            "# task\n```yaml\nstatus: validating\nissue: 10\npr: 20\n```\n",
        )
        self.write(
            "docs/agents/tasks/active/OTV2-closed-issue.md",
            "# task\n```yaml\nstatus: waiting\nissue: 11\npr: null\n```\n",
        )
        payloads = {
            "issues/10": {"state": "open"},
            "issues/11": {"state": "closed"},
            "pulls/20": {"state": "closed", "merged_at": "2026-09-21T00:00:00Z"},
        }

        def request(url: str) -> object:
            return payloads[url.rsplit("/", 2)[-2] + "/" + url.rsplit("/", 1)[-1]]

        errors = validator.validate_active_task_live_state(request)
        self.assertIn(
            "active task packet docs/agents/tasks/active/OTV2-merged-pr.md names terminal canonical PR #20 (merged)",
            errors,
        )
        self.assertIn(
            "active task packet docs/agents/tasks/active/OTV2-closed-issue.md names closed Issue #11 without an open canonical PR",
            errors,
        )

    def test_active_task_live_state_accepts_open_pr_for_closed_issue(self) -> None:
        self.write(
            "docs/agents/tasks/active/OTV2-open-pr.md",
            "# task\n```yaml\nstatus: validating\nissue: 12\npr: 21\n```\n",
        )

        def request(url: str) -> object:
            if "/pulls/" in url:
                return {"state": "open", "merged_at": None}
            return {"state": "closed"}

        self.assertEqual(validator.validate_active_task_live_state(request), [])

        self.write(
            "docs/agents/tasks/active/OTV2-unrelated-terminal.md",
            "# task\n```yaml\nstatus: validating\nissue: 31\npr: 41\n```\n",
        )

        requested: list[str] = []

        def scoped_request(url: str) -> object:
            requested.append(url)
            if url.endswith("/pulls/21"):
                return {"state": "open", "merged_at": None}
            if url.endswith("/issues/12"):
                return {"state": "closed"}
            if url.endswith("/pulls/41"):
                return {"state": "closed", "merged_at": "2026-09-22T00:00:00Z"}
            if url.endswith("/issues/31"):
                return {"state": "open"}
            raise AssertionError(url)

        self.assertEqual(
            validator.validate_active_task_live_state(
                scoped_request,
                {"docs/agents/tasks/active/OTV2-open-pr.md"},
            ),
            [],
        )
        self.assertEqual(len(requested), 2)
        self.assertTrue(all(url.endswith(("/pulls/21", "/issues/12")) for url in requested))


if __name__ == "__main__":
    unittest.main()
