from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[3]
MODULE_PATH = ROOT / "tools/agents/validate_inherited_prompt_policy.py"
spec = importlib.util.spec_from_file_location("meta_adoption", MODULE_PATH)
assert spec and spec.loader
adoption = importlib.util.module_from_spec(spec)
spec.loader.exec_module(adoption)

SHA = "a" * 40
MAIN = "b" * 40


class CentralStatementView:
    @staticmethod
    def _statements(text: str) -> list[str]:
        return [line.strip() for line in text.splitlines() if line.strip()]

    @staticmethod
    def _is_audit_or_negative(statement: str) -> bool:
        return statement.casefold().startswith(("do not ", "remove ", "retire ", "audit ", "inspect "))


class MetaPolicyAdoptionTests(unittest.TestCase):
    def test_authenticates_exact_ancestor_of_protected_main(self):
        responses = [
            {"sha": SHA},
            {"protected": True, "commit": {"sha": MAIN}},
            {"status": "ahead", "base_commit": {"sha": SHA}, "merge_base_commit": {"sha": SHA}},
        ]
        with mock.patch.object(adoption, "_request", side_effect=responses):
            self.assertEqual(
                adoption._authenticate_binding({"authority_repository": "Oteryn/Oteryn", "authority_commit": SHA}),
                ("Oteryn/Oteryn", SHA, MAIN),
            )

    def test_rejects_unprotected_main(self):
        responses = [{"sha": SHA}, {"protected": False, "commit": {"sha": MAIN}}]
        with mock.patch.object(adoption, "_request", side_effect=responses):
            with self.assertRaisesRegex(ValueError, "not verified as protected"):
                adoption._authenticate_binding({"authority_repository": "Oteryn/Oteryn", "authority_commit": SHA})

    def test_rejects_compare_coordinates_for_another_commit(self):
        responses = [
            {"sha": SHA},
            {"protected": True, "commit": {"sha": MAIN}},
            {"status": "ahead", "base_commit": {"sha": "c" * 40}, "merge_base_commit": {"sha": SHA}},
        ]
        with mock.patch.object(adoption, "_request", side_effect=responses):
            with self.assertRaisesRegex(ValueError, "exact commit"):
                adoption._authenticate_binding({"authority_repository": "Oteryn/Oteryn", "authority_commit": SHA})

    def test_game_bootstrap_preserves_domain_invariants_and_binding_delivery(self):
        text = (ROOT / "AGENTS.md").read_text(encoding="utf-8")
        for value in (
            "docs/agents/META_AGENT_POLICY_BINDING.json",
            "Oteryn/Oteryn-Game",
            "protocol-oteryn",
            "multichannel",
            "WorldId",
            "ChannelId",
            "session-generation",
            "repository-native GitHub APIs",
            "isolated checkout or worktree",
            "Remote Desktop is denied",
        ):
            self.assertIn(value, text)

    def test_local_routing_extensions_use_bound_states_and_runtime_configuration(self):
        contract = json.loads((ROOT / "docs/agents/GOVERNANCE_CONTRACT.json").read_text(encoding="utf-8"))
        self.assertEqual(
            contract["terminal_invocation_results"],
            ["DONE", "WAITING_EXTERNAL", "BLOCKED", "STALLED"],
        )
        for relative in (
            "docs/agents/programs/OTERYN_GAME_AGENT_OPERATOR_RUNBOOK.md",
            "docs/agents/programs/OTERYN_V2_TERRA_SOL_EXECUTION_SCHEDULER.md",
            "docs/agents/prompts/OTV2_OWNER_EXECUTION_STATUS_ADVISOR.md",
        ):
            text = (ROOT / relative).read_text(encoding="utf-8")
            self.assertNotIn("GPT-5.6", text, relative)
            self.assertNotIn("highest available", text, relative)

    def test_reusable_prompts_keep_aliases_and_remove_execution_configuration(self):
        lifecycle = json.loads((ROOT / "docs/agents/PROMPT_LIFECYCLE.json").read_text(encoding="utf-8"))
        paths = adoption._reusable_prompt_paths(lifecycle)
        self.assertTrue(paths)
        for relative in paths:
            text = (ROOT / relative).read_text(encoding="utf-8")
            self.assertNotIn("recommended_model:", text)
            self.assertNotIn("recommended_effort:", text)
            self.assertNotIn("## Remote Desktop execution routing", text)
            self.assertNotIn("## Canonical Codex review routing", text)
            self.assertEqual(adoption._legacy_review_controller_errors(text, CentralStatementView), [], relative)

    def test_workflow_authenticates_the_bound_meta_consumer(self):
        workflow = (ROOT / ".github/workflows/agent-governance.yml").read_text(encoding="utf-8")
        step = workflow.split("- name: Validate bound META policy and task prompts", 1)[1]
        step = step.split("\n      - name:", 1)[0]
        self.assertIn("GITHUB_TOKEN: ${{ github.token }}", step)
        self.assertIn("python tools/agents/validate_inherited_prompt_policy.py", step)

    def test_rejects_operative_retired_review_controller(self):
        for statement in (
            "Apply CODEX_REVIEW_POLICY.json mechanically.",
            "When CODEX_REQUIRED, block the candidate.",
            "Select CODEX_NOT_REQUIRED_BY_THIS_POLICY.",
        ):
            self.assertEqual(
                len(adoption._legacy_review_controller_errors(statement, CentralStatementView)),
                1,
                statement,
            )

    def test_allows_negative_and_unrelated_review_prose(self):
        text = "\n".join((
            "Do not use CODEX_REQUIRED; it is retired.",
            "Remove CODEX_REVIEW_POLICY.json as an active controller.",
            "Owner-funded review controls permission only.",
            "A covered review is historical evidence.",
        ))
        self.assertEqual(adoption._legacy_review_controller_errors(text, CentralStatementView), [])

    def test_current_review_policy_consumers_do_not_use_retired_controller(self):
        consumers = (*adoption.CURRENT_REVIEW_POLICY_CONSUMERS, *adoption.CURRENT_ACTIVE_REVIEW_CONSUMERS)
        for relative in consumers:
            path = ROOT / relative
            if not path.is_file():
                continue
            text = path.read_text(encoding="utf-8")
            self.assertEqual(
                adoption._legacy_review_controller_errors(text, CentralStatementView), [], relative,
            )

    def test_representative_prompts_retain_scope_and_domain_acceptance(self):
        cases = {
            "docs/agents/prompts/OTV2_IMPL_DOMAIN_CORE.md": ("ItemDefinition", "No persistence implementation"),
            "docs/agents/prompts/OTV2_IMPL_DURABILITY.md": ("PostgreSQL", "session"),
            "docs/agents/prompts/OTV2_IMPL_GAME_CHANNEL.md": ("ChannelId", "multichannel"),
            "docs/agents/prompts/OTV2_IMPL_NATIVE_CLIENT.md": ("GameSession", "protocol-oteryn"),
        }
        for relative, required in cases.items():
            text = (ROOT / relative).read_text(encoding="utf-8")
            for value in required:
                self.assertIn(value, text, relative)


if __name__ == "__main__":
    unittest.main()
