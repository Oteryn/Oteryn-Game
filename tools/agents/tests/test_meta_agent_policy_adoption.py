from __future__ import annotations

import importlib.util
import json
import os
import tempfile
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

    def test_pull_request_live_task_scope_is_candidate_bound(self):
        base = "c" * 40
        head = "d" * 40
        pull = {
            "state": "open",
            "head": {"sha": head, "repo": {"full_name": "Oteryn/Oteryn-Game"}},
            "base": {"sha": base, "ref": "main"},
            "changed_files": 2,
        }
        files = [
            {
                "filename": "docs/agents/tasks/active/OTV2-selected.md",
                "status": "modified",
            },
            {
                "filename": "apps/game-server/src/lib.rs",
                "status": "modified",
            },
        ]

        def request(url: str):
            if "/files?" in url:
                return files
            if url.endswith("/pulls/77"):
                return pull
            raise AssertionError(url)

        with mock.patch.object(adoption, "_request", side_effect=request):
            self.assertEqual(
                adoption._pull_request_active_task_paths(77, head),
                {"docs/agents/tasks/active/OTV2-selected.md"},
            )

    def test_pull_request_live_task_scope_rejects_malformed_file_paths(self):
        base = "4" * 40
        head = "5" * 40
        pull = {
            "state": "open",
            "head": {"sha": head, "repo": {"full_name": "Oteryn/Oteryn-Game"}},
            "base": {"sha": base, "ref": "main"},
            "changed_files": 1,
        }
        malformed = (
            {},
            {"filename": ""},
            {"filename": 7},
            {"filename": "../docs/agents/tasks/active/OTV2-selected.md"},
            {
                "filename": "apps/game-server/src/lib.rs",
                "previous_filename": "../docs/agents/tasks/active/OTV2-selected.md",
            },
        )

        for file_record in malformed:
            with self.subTest(file_record=file_record):
                def request(url: str):
                    if "/files?" in url:
                        return [file_record]
                    if url.endswith("/pulls/79"):
                        return pull
                    raise AssertionError(url)

                with mock.patch.object(adoption, "_request", side_effect=request):
                    with self.assertRaisesRegex(ValueError, "invalid pull request"):
                        adoption._pull_request_active_task_paths(79, head)

    def test_pull_request_live_task_scope_rejects_duplicate_filenames(self):
        base = "6" * 40
        head = "7" * 40
        pull = {
            "state": "open",
            "head": {"sha": head, "repo": {"full_name": "Oteryn/Oteryn-Game"}},
            "base": {"sha": base, "ref": "main"},
            "changed_files": 2,
        }
        duplicate = {"filename": "apps/game-server/src/lib.rs", "status": "modified"}

        def request(url: str):
            if "/files?" in url:
                return [duplicate, duplicate.copy()]
            if url.endswith("/pulls/80"):
                return pull
            raise AssertionError(url)

        with mock.patch.object(adoption, "_request", side_effect=request):
            with self.assertRaisesRegex(ValueError, "duplicate pull request filename"):
                adoption._pull_request_active_task_paths(80, head)

    def test_pull_request_live_task_scope_rejects_mid_read_base_drift(self):
        first_base = "1" * 40
        second_base = "2" * 40
        head = "3" * 40
        pulls = [
            {
                "state": "open",
                "head": {"sha": head, "repo": {"full_name": "Oteryn/Oteryn-Game"}},
                "base": {"sha": first_base, "ref": "main"},
                "changed_files": 1,
            },
            {
                "state": "open",
                "head": {"sha": head, "repo": {"full_name": "Oteryn/Oteryn-Game"}},
                "base": {"sha": second_base, "ref": "main"},
                "changed_files": 1,
            },
        ]
        files = [{"filename": "docs/agents/tasks/active/OTV2-selected.md", "status": "modified"}]

        def request(url: str):
            if "/files?" in url:
                return files
            if url.endswith("/pulls/78"):
                return pulls.pop(0)
            raise AssertionError(url)

        with mock.patch.object(adoption, "_request", side_effect=request):
            with self.assertRaisesRegex(ValueError, "moved during live-state candidate validation"):
                adoption._pull_request_active_task_paths(78, head)

    def test_active_task_live_scope_uses_exact_event_head_not_historical_base(self):
        base = "e" * 40
        head = "f" * 40
        event = {
            "number": 88,
            "pull_request": {
                "number": 88,
                "head": {"sha": head},
                "base": {"sha": base},
            },
        }
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "event.json"
            path.write_text(json.dumps(event), encoding="utf-8")
            with mock.patch.dict(
                os.environ,
                {
                    "GITHUB_EVENT_NAME": "pull_request",
                    "GITHUB_EVENT_PATH": str(path),
                },
                clear=False,
            ), mock.patch.object(
                adoption,
                "_pull_request_active_task_paths",
                return_value={"docs/agents/tasks/active/OTV2-selected.md"},
            ) as scoped:
                self.assertEqual(
                    adoption._active_task_live_scope(),
                    {"docs/agents/tasks/active/OTV2-selected.md"},
                )
                scoped.assert_called_once_with(88, head)

    def test_non_pr_live_task_scope_keeps_full_health_scan(self):
        with mock.patch.dict(os.environ, {"GITHUB_EVENT_NAME": "push"}, clear=False):
            self.assertIsNone(adoption._active_task_live_scope())

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
            "Repository-native API authoring is independently valid",
            "bounded sequential high-level file mutations may be used before candidate freeze",
            "Remote Desktop is denied",
        ):
            self.assertIn(value, text)
        self.assertNotIn("The only no-worktree tracked-file publication exception", text)

    def test_publication_integrity_provider_contract_distinguishes_authoring_from_reconstruction(self):
        binding = json.loads((ROOT / "docs/agents/META_AGENT_POLICY_BINDING.json").read_text(encoding="utf-8"))
        self.assertEqual(binding["authority_commit"], PUBLICATION_INTEGRITY_AUTHORITY)
        text = (ROOT / "docs/agents/AGENTS.md").read_text(encoding="utf-8")
        for value in (
            "Repository-native API authoring may use bounded sequential high-level file mutations",
            "expected_final_authoring_head",
            "Fresh live readback must equal that exact SHA",
            "writer/state drift",
            "A separately allocated API-native publication may instead select a **new candidate**",
            "one server-side mutation atomically fences the exact expected task-branch head",
            "Never use sequential API writes to reconstruct a selected local candidate",
            "ancestry-only `force=false` ref movement",
            "raw Git Data reconstruction",
        ):
            self.assertIn(value, text)

    def test_work_coordinator_preflights_mutating_execution_before_dispatch(self):
        root_text = (ROOT / "AGENTS.md").read_text(encoding="utf-8")
        for value in (
            "BLOCKED_CAPABILITY_UNAVAILABLE",
            "Missing repository workspace, Git CLI or push capability is not a Remote Desktop exception",
            "Repository-native API authoring is independently valid",
            "bounded sequential high-level file mutations may be used before candidate freeze",
            "single server-side mutation atomically fences the exact expected task-branch head",
        ):
            self.assertIn(value, root_text)

        coordinator = (ROOT / "docs/agents/prompts/OTV2_WORK_DELIVERY_COORDINATOR.md").read_text(
            encoding="utf-8"
        )
        for value in (
            "## Execution-capability preflight",
            "Before dispatching any mutating worker",
            "`api_native_authoring`",
            "`atomic_api_candidate`",
            "Fresh-read the live branch head before each mutation",
            "expected_final_authoring_head",
            "require it to equal `expected_final_authoring_head` before freeze",
            "writer/state drift",
            "freeze that exact fenced remote head as the candidate",
            "Do not ask the owner for Remote Desktop merely to obtain",
            "execution_route: <isolated_git | api_native_authoring | atomic_api_candidate | read_only>",
            "publication_route: <guarded_git | frozen_api_authored_head | atomic_expected_head_api | none>",
            "For every concrete entry in `required_validation`",
            "capability: <PROVEN | UNKNOWN>",
            "or every required-validation route",
            "reconstruction of an existing candidate through sequential per-file Contents writes",
        ):
            self.assertIn(value, coordinator)

        lifecycle = json.loads((ROOT / "docs/agents/PROMPT_LIFECYCLE.json").read_text(encoding="utf-8"))
        entry = next(
            prompt for prompt in lifecycle["prompts"] if prompt["prompt_id"] == "OTV2_WORK_DELIVERY_COORDINATOR"
        )
        self.assertEqual(entry["version"], "2.2")
        self.assertNotIn(
            "compact execution profile over `docs/agents/prompts/OTV2_IMPLEMENTATION_COORDINATOR.md`",
            coordinator,
        )
        self.assertIn(
            "Authority comes directly from protected root/nearest `AGENTS.md`",
            coordinator,
        )

        durability = (ROOT / "docs/agents/prompts/OTV2_IMPL_DURABILITY.md").read_text(encoding="utf-8")
        for value in (
            "bounded `api_native_authoring`",
            "sequential high-level file mutations are WIP",
            "expected_final_authoring_head",
            "require equality before freeze",
            "writer/state drift",
            "freeze that fenced remote head as the candidate",
            "atomic expected-head API **new-candidate** publication",
            "ancestry-only `force=false` ref movement",
            "reconstruct a selected local Git candidate through sequential API writes",
        ):
            self.assertIn(value, durability)

        closure = (ROOT / "docs/agents/CLOSURE_CONVERGENCE_PROTOCOL.md").read_text(encoding="utf-8")
        for value in (
            "bounded `api_native_authoring`",
            "sequential high-level file writes are WIP only",
            "expected_final_authoring_head",
            "require equality before freeze",
            "writer/state drift",
            "freeze that fenced remote head as the candidate",
            "atomic expected-head API **new-candidate** publication",
            "precondition mismatch must create no commit and move no branch",
            "ancestry-only `force=false` ref movement",
            "sequential API writes to reconstruct a selected local candidate",
        ):
            self.assertIn(value, closure)

        durability_entry = next(
            prompt for prompt in lifecycle["prompts"] if prompt["prompt_id"] == "OTV2_IMPL_DURABILITY"
        )
        self.assertEqual(durability_entry["version"], "1.4")

    def test_work_is_single_control_plane_and_startups_are_targeted(self):
        lifecycle = json.loads((ROOT / "docs/agents/PROMPT_LIFECYCLE.json").read_text(encoding="utf-8"))
        entries = {entry["prompt_id"]: entry for entry in lifecycle["prompts"]}

        terra = entries["OTV2_TERRA_GAME_CONTROL_PLANE"]
        self.assertEqual(terra["version"], "1.2")
        self.assertEqual(terra["status"], "retired")
        self.assertIs(terra["reusable"], False)
        self.assertEqual(
            terra["superseded_by"],
            "docs/agents/prompts/OTV2_WORK_DELIVERY_COORDINATOR.md",
        )

        implementation = entries["OTV2_IMPLEMENTATION_COORDINATOR"]
        self.assertEqual(implementation["version"], "1.2")
        self.assertEqual(implementation["status"], "retired")
        self.assertIs(implementation["reusable"], False)
        self.assertEqual(
            implementation["superseded_by"],
            "docs/agents/prompts/OTV2_WORK_DELIVERY_COORDINATOR.md",
        )

        retired_to_implementation = [
            entry["prompt_id"]
            for entry in lifecycle["prompts"]
            if entry.get("superseded_by") == "docs/agents/prompts/OTV2_IMPLEMENTATION_COORDINATOR.md"
        ]
        self.assertEqual(retired_to_implementation, [])

        for prompt_id, version in {
            "OTV2_REFERENCE_INVESTIGATOR": "1.1",
            "OTV2_OWNER_EXECUTION_STATUS_ADVISOR": "1.1",
            "OTV2_CONTENT_WORLD_INDEPENDENT_AUDIT": "1.1",
            "OTV2_DEFECT_DISCOVERY_SUPERVISOR": "1.1",
            "OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR": "1.2",
            "OTV2_WORK_DELIVERY_INDEPENDENT_AUDITOR": "1.5",
            "OTV2_INDEPENDENT_PROGRAMME_ARCHITECTURE_AUDIT": "1.3",
        }.items():
            self.assertEqual(entries[prompt_id]["version"], version)

        readme = (ROOT / "docs/agents/prompts/README.md").read_text(encoding="utf-8")
        self.assertNotIn(
            "`OTV2_TERRA_GAME_CONTROL_PLANE.md` — deterministic Game control plane",
            readme,
        )

        self.assertIn("former `Oteryn: terra game coordinator` and `Oteryn: implementation coordinator` profiles are retired", readme)

        implementation_prompt = (ROOT / "docs/agents/prompts/OTV2_IMPLEMENTATION_COORDINATOR.md").read_text(
            encoding="utf-8"
        )
        self.assertIn("RETIRED / PROVENANCE ONLY", implementation_prompt)
        self.assertIn("MUTATION_AUTHORITY: NONE", implementation_prompt)

        runbook = (ROOT / "docs/agents/programs/OTERYN_GAME_AGENT_OPERATOR_RUNBOOK.md").read_text(encoding="utf-8")
        self.assertNotIn("| `Oteryn: terra game coordinator` |", runbook)
        self.assertIn("exactly one active mutating control plane in ChatGPT Work: `Oteryn: work coordinator`", runbook)

        scheduler = (ROOT / "docs/agents/programs/OTERYN_V2_TERRA_SOL_EXECUTION_SCHEDULER.md").read_text(encoding="utf-8")
        self.assertIn("# Oteryn v2 Work + Sol Execution Scheduler", scheduler)
        self.assertNotIn("`Oteryn: terra game coordinator`", scheduler)
        self.assertNotIn("Work/Terra", scheduler)

        reference = (ROOT / "docs/agents/prompts/OTV2_REFERENCE_INVESTIGATOR.md").read_text(encoding="utf-8")
        self.assertIn("Resolve the requested `<lane>` exactly first", reference)
        self.assertIn("full `PROMPT_LIFECYCLE.json`", reference)
        self.assertIn("not ordinary invocation prerequisites", reference)

        owner = (ROOT / "docs/agents/prompts/OTV2_OWNER_EXECUTION_STATUS_ADVISOR.md").read_text(encoding="utf-8")
        self.assertIn("matching lifecycle entry", owner)
        self.assertIn("Terra is retired", owner)

        content_audit = (ROOT / "docs/agents/prompts/OTV2_CONTENT_WORLD_INDEPENDENT_AUDIT.md").read_text(encoding="utf-8")
        self.assertIn("matching `OTV2_CONTENT_WORLD_INDEPENDENT_AUDIT` lifecycle entry", content_audit)

        work_audit = (ROOT / "docs/agents/prompts/OTV2_WORK_DELIVERY_INDEPENDENT_AUDITOR.md").read_text(encoding="utf-8")
        self.assertIn("`PROMPT_EVAL_STANDARD.md` is needed only when prompt/harness behavior is an audit target", work_audit)
        self.assertNotIn("Work/Terra", work_audit)
        self.assertNotIn("OTV2_IMPLEMENTATION_COORDINATOR.md", work_audit)
        self.assertIn("Work-only Game control plane", work_audit)

        programme_audit = (ROOT / "docs/agents/prompts/OTV2_INDEPENDENT_PROGRAMME_ARCHITECTURE_AUDIT.md").read_text(encoding="utf-8")
        self.assertIn("intentionally a **whole-programme audit**", programme_audit)
        self.assertIn('prompt_version: "1.3"', programme_audit)

        terra_prompt = (ROOT / "docs/agents/prompts/OTV2_TERRA_GAME_CONTROL_PLANE.md").read_text(
            encoding="utf-8"
        )
        self.assertIn('prompt_version: "1.2"', terra_prompt)

    def test_api_native_authoring_never_escalates_missing_git_to_remote_desktop(self):
        root_text = (ROOT / "AGENTS.md").read_text(encoding="utf-8")
        coordinator = (ROOT / "docs/agents/prompts/OTV2_WORK_DELIVERY_COORDINATOR.md").read_text(
            encoding="utf-8"
        )
        self.assertIn(
            "do not request Remote Desktop merely to obtain those capabilities",
            root_text,
        )
        self.assertIn(
            "Missing local Git capability is not a Remote Desktop exception",
            coordinator,
        )
        self.assertIn(
            "when either `api_native_authoring` or `atomic_api_candidate`",
            coordinator,
        )
        self.assertIn(
            "Bounded sequential high-level Contents/API authoring is valid only before freeze",
            coordinator,
        )

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
