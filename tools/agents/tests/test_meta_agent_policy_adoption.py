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
PUBLICATION_INTEGRITY_AUTHORITY = "e102056cc4b9219bc482ceb05afebeb4d62b7bc8"


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
            "isolated checkout or worktree for local-Git tracked-file mutation",
            "only no-worktree tracked-file publication exception",
            "Remote Desktop is denied",
        ):
            self.assertIn(value, text)

    def test_publication_integrity_provider_contract_fails_closed_without_reconstruction(self):
        binding = json.loads((ROOT / "docs/agents/META_AGENT_POLICY_BINDING.json").read_text(encoding="utf-8"))
        self.assertEqual(binding["authority_commit"], PUBLICATION_INTEGRITY_AUTHORITY)
        text = (ROOT / "docs/agents/AGENTS.md").read_text(encoding="utf-8")
        for value in (
            "API-native publication may select a **new candidate**",
            "one server-side mutation atomically fences the exact expected task-branch head",
            "candidate-specific evidence from a superseded head is not reusable",
            "ancestry-only `force=false` ref movement",
            "raw Git Data reconstruction",
            "sequential per-file API reconstruction",
        ):
            self.assertIn(value, text)

    def test_work_coordinator_preflights_mutating_execution_before_dispatch(self):
        root_text = (ROOT / "AGENTS.md").read_text(encoding="utf-8")
        for value in (
            "BLOCKED_CAPABILITY_UNAVAILABLE",
            "Missing repository workspace, Git CLI or push capability is not a Remote Desktop exception",
            "API-native **new-candidate** route",
            "single server-side mutation atomically fences the exact expected task-branch head",
        ):
            self.assertIn(value, root_text)

        coordinator = (ROOT / "docs/agents/prompts/OTV2_WORK_DELIVERY_COORDINATOR.md").read_text(
            encoding="utf-8"
        )
        for value in (
            "## Execution-capability preflight",
            "Before dispatching any mutating worker",
            "isolated checkout/worktree",
            "atomic expected-head API new-candidate publication",
            "BLOCKED_CAPABILITY_UNAVAILABLE",
            "Do not ask the owner for Remote Desktop merely to obtain",
            "execution_route: <isolated_git | api_native | read_only>",
            "publication_route: <guarded_git | atomic_expected_head_api | none>",
            "required-validation",
            "For every concrete entry in `required_validation`",
            "capability: <PROVEN | UNKNOWN>",
            "or every required-validation route",
        ):
            self.assertIn(value, coordinator)

        lifecycle = json.loads((ROOT / "docs/agents/PROMPT_LIFECYCLE.json").read_text(encoding="utf-8"))
        entry = next(
            prompt for prompt in lifecycle["prompts"] if prompt["prompt_id"] == "OTV2_WORK_DELIVERY_COORDINATOR"
        )
        self.assertEqual(entry["version"], "1.9")

        durability = (ROOT / "docs/agents/prompts/OTV2_IMPL_DURABILITY.md").read_text(encoding="utf-8")
        for value in (
            "atomic expected-head API **new-candidate** publication",
            "one server-side mutation fences the exact expected task-branch predecessor",
            "fresh candidate-specific validation/review evidence",
            "ancestry-only `force=false` ref movement",
            "sequential per-file API commits",
        ):
            self.assertIn(value, durability)

        closure = (ROOT / "docs/agents/CLOSURE_CONVERGENCE_PROTOCOL.md").read_text(encoding="utf-8")
        for value in (
            "atomic expected-head API **new-candidate** publication",
            "candidate-specific validation/review evidence from any superseded head is not reusable",
            "precondition mismatch must create no commit and move no branch",
            "ancestry-only `force=false` ref movement",
            "sequential per-file API commits",
        ):
            self.assertIn(value, closure)

        durability_entry = next(
            prompt for prompt in lifecycle["prompts"] if prompt["prompt_id"] == "OTV2_IMPL_DURABILITY"
        )
        self.assertEqual(durability_entry["version"], "1.3")

    def test_owner_funded_review_standing_authorization_is_bounded_and_deduplicated(self):
        policy = (ROOT / "docs/agents/OWNER_FUNDED_AI_POLICY.md").read_text(encoding="utf-8")
        for value in (
            "## Standing repository review authorization",
            "one external independent",
            "survives chat, worker, coordinator and task-phase handoffs",
            "Do not ask the owner again for a covered review",
            "already requested, running or completed",
            "does **not** cover optional/speculative extra reviews",
            "### Single review-dispatch owner",
            "only the unique active",
            "must not emit the trigger itself",
        ):
            self.assertIn(value, policy)

        root_text = (ROOT / "AGENTS.md").read_text(encoding="utf-8")
        for value in (
            "bounded standing authorization for required external review",
            "Do not ask the owner again for a covered review",
            "one-writer control-plane action",
            "direct workers return a review packet instead of emitting it",
        ):
            self.assertIn(value, root_text)

        coordinator = (ROOT / "docs/agents/prompts/OTV2_WORK_DELIVERY_COORDINATOR.md").read_text(
            encoding="utf-8"
        )
        for value in (
            "review_authorization: <standing_required_review | task_specific | none>",
            "review_trigger_owner: <control_plane | standalone_task_owner | none>",
            "review_request_state: <not_requested | running | completed | stale>",
            "## Review authorization, ownership and de-duplication",
            "do **not** ask the owner again",
            "Workers may return a complete review packet, but they must not emit",
            "Ambiguous/slow provider response is a readback problem",
        ):
            self.assertIn(value, coordinator)

        readme = (ROOT / "docs/agents/prompts/README.md").read_text(encoding="utf-8")
        self.assertIn("survives chat/worker handoffs", readme)
        self.assertIn("A direct worker never emits the owner-funded review trigger", readme)
        self.assertIn("unique active control plane", readme)

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
