from __future__ import annotations

import re
import sys
import unittest
from pathlib import Path


REPOSITORY_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPOSITORY_ROOT / "tools" / "architecture"))

import semantic_contract_audit as audit  # noqa: E402


class SemanticContractAuditDispatchTests(unittest.TestCase):
    def test_selects_profile_for_one_relevant_file(self) -> None:
        self.assertEqual(
            audit.select_profiles(
                {"docs/agents/tasks/archive/OTV2-20260815-alpha-client-architecture.md"}
            ),
            ["ALPHA_CLIENT_01"],
        )

    def test_unrelated_file_does_not_suppress_profile(self) -> None:
        self.assertEqual(
            audit.select_profiles(audit.F_PATHS | {"README.md"}),
            ["ANL_02_ANL_03"],
        )

    def test_selects_every_affected_profile(self) -> None:
        self.assertEqual(
            audit.select_profiles(
                {
                    "docs/architecture/ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_ANALYSIS.md",
                    "apps/game-server/src/foundation/fnd04_verifier.rs",
                }
            ),
            ["ALPHA_CLIENT_01", "FOUNDATION_RECONNECT_DURABILITY_V1"],
        )

    def test_unrelated_only_is_not_applicable(self) -> None:
        self.assertEqual(audit.select_profiles({"README.md"}), [])

    def test_profile_order_is_stable_for_unordered_input(self) -> None:
        self.assertEqual(
            audit.select_profiles(audit.R_PATHS | audit.F_PATHS | audit.E_PATHS),
            [
                "ALPHA_CLIENT_01",
                "ANL_02_ANL_03",
                "FOUNDATION_RECONNECT_DURABILITY_V1",
            ],
        )

    def test_current_repository_profiles_execute_against_real_inputs(self) -> None:
        for profile_check in (audit.alpha, audit.analytics, audit.foundation_reconnect):
            with self.subTest(profile=profile_check.__name__):
                self.assertTrue(profile_check())

    def test_failure_does_not_suppress_later_selected_profile(self) -> None:
        calls = []

        def fails() -> list[str]:
            calls.append("first")
            raise SystemExit("first failed")

        def passes() -> list[str]:
            calls.append("second")
            return ["second check"]

        profiles, failures = audit.run_profiles(
            ["FIRST", "SECOND"], {"FIRST": fails, "SECOND": passes}
        )

        self.assertEqual(calls, ["first", "second"])
        self.assertEqual([profile["verdict"] for profile in profiles], ["FAIL", "PASS"])
        self.assertEqual(failures, ["FIRST: first failed"])

    def test_foundation_oracle_rejects_each_relaxed_authority_comparison(self) -> None:
        task, implementation, verifier = self._foundation_documents()
        authority_matcher = audit.rust_braced_block(
            implementation,
            "fn current_authority_matches_record(",
            "complete current authority matcher",
        )
        for comparison in audit.CURRENT_AUTHORITY_COMPARISONS:
            with self.subTest(comparison=comparison):
                pattern = re.sub(r"\\ ", r"\\s+", re.escape(comparison))
                mutated_matcher, replacements = re.subn(
                    pattern, "true", authority_matcher, count=1
                )
                self.assertEqual(replacements, 1)
                mutated = implementation.replace(
                    authority_matcher, mutated_matcher, 1
                )
                with self.assertRaisesRegex(
                    SystemExit, "complete current authority matcher"
                ):
                    audit.foundation_reconnect_documents(task, mutated, verifier)

    def test_foundation_oracle_rejects_missing_v1_record_equality(self) -> None:
        task, implementation, verifier = self._foundation_documents()
        v1 = audit.rust_impl_method(
            implementation,
            "impl ReconnectDurabilityFlowV1 {",
            "pub fn accept_reconciliation(",
            "V1 reconciliation",
        )
        v2 = audit.rust_impl_method(
            implementation,
            "impl ReconnectDurabilityFlowV2 {",
            "pub fn accept_reconciliation(",
            "V2 reconciliation",
        )
        mutated_v1 = self._replace_compact_fragment(v1, "if snapshot.record != self.record")
        mutated = implementation.replace(v1, mutated_v1, 1)
        self.assertIn("if snapshot.record != self.record", audit.compact(v2))
        with self.assertRaisesRegex(SystemExit, "FlowV1 accept_reconciliation"):
            audit.foundation_reconnect_documents(task, mutated, verifier)

    def test_foundation_oracle_rejects_each_relaxed_evidence_comparison(self) -> None:
        task, implementation, verifier = self._foundation_documents()
        evidence = audit.rust_braced_block(
            implementation,
            "fn authenticated_evidence_observed_by(",
            "authenticated evidence observation matcher",
        )
        for comparison in audit.AUTHENTICATED_EVIDENCE_COMPARISONS:
            with self.subTest(comparison=comparison):
                mutated_evidence = self._replace_compact_fragment(evidence, comparison)
                mutated = implementation.replace(evidence, mutated_evidence, 1)
                with self.assertRaisesRegex(
                    SystemExit, "authenticated evidence observation matcher"
                ):
                    audit.foundation_reconnect_documents(task, mutated, verifier)

    def test_foundation_oracle_rejects_each_authorize_invariant_per_version(self) -> None:
        task, implementation, verifier = self._foundation_documents()
        for version in ("V1", "V2"):
            method = audit.rust_impl_method(
                implementation,
                f"impl ReconnectDurabilityFlow{version} {{",
                "pub fn authorize_commit(",
                f"{version} authorize_commit",
            )
            for invariant in audit.AUTHORIZE_COMMIT_ORDERED_INVARIANTS:
                with self.subTest(version=version, invariant=invariant):
                    mutated_method = self._replace_compact_fragment(method, invariant)
                    mutated = implementation.replace(method, mutated_method, 1)
                    with self.assertRaisesRegex(
                        SystemExit, f"Flow{version} authorize_commit"
                    ):
                        audit.foundation_reconnect_documents(task, mutated, verifier)

    def test_foundation_oracle_rejects_early_commit_request_per_version(self) -> None:
        task, implementation, verifier = self._foundation_documents()
        request = audit.AUTHORIZE_COMMIT_ORDERED_INVARIANTS[-1]
        for version in ("V1", "V2"):
            method = audit.rust_impl_method(
                implementation,
                f"impl ReconnectDurabilityFlow{version} {{",
                "pub fn authorize_commit(",
                f"{version} authorize_commit",
            )
            compact_method = audit.compact(method)
            without_request = compact_method.replace(request, "", 1)
            opening = without_request.index("{") + 1
            reordered = (
                without_request[:opening]
                + " "
                + request
                + " "
                + without_request[opening:]
            )
            mutated = implementation.replace(method, reordered, 1)
            with self.subTest(version=version):
                with self.assertRaisesRegex(
                    SystemExit, f"Flow{version} authorize_commit"
                ):
                    audit.foundation_reconnect_documents(task, mutated, verifier)

    def test_foundation_oracle_rejects_each_reconciliation_invariant_per_version(self) -> None:
        task, implementation, verifier = self._foundation_documents()
        versions = (
            ("V1", audit.RECONCILIATION_V1_ORDERED_INVARIANTS),
            ("V2", audit.RECONCILIATION_V2_ORDERED_INVARIANTS),
        )
        for version, invariants in versions:
            method = audit.rust_impl_method(
                implementation,
                f"impl ReconnectDurabilityFlow{version} {{",
                "pub fn accept_reconciliation(",
                f"{version} accept_reconciliation",
            )
            for invariant in invariants:
                with self.subTest(version=version, invariant=invariant):
                    mutated_method = self._replace_compact_fragment(method, invariant)
                    mutated = implementation.replace(method, mutated_method, 1)
                    with self.assertRaisesRegex(
                        SystemExit, f"Flow{version} accept_reconciliation"
                    ):
                        audit.foundation_reconnect_documents(task, mutated, verifier)

    def test_foundation_oracle_rejects_early_controller_install_per_version(self) -> None:
        task, implementation, verifier = self._foundation_documents()
        versions = (
            ("V1", audit.RECONCILIATION_V1_ORDERED_INVARIANTS),
            ("V2", audit.RECONCILIATION_V2_ORDERED_INVARIANTS),
        )
        for version, invariants in versions:
            method = audit.rust_impl_method(
                implementation,
                f"impl ReconnectDurabilityFlow{version} {{",
                "pub fn accept_reconciliation(",
                f"{version} accept_reconciliation",
            )
            compact_method = audit.compact(method)
            controller = invariants[-3]
            committed = invariants[2]
            without_controller = compact_method.replace(controller, "", 1)
            insertion = without_controller.index(committed) + len(committed)
            reordered = (
                without_controller[:insertion]
                + " "
                + controller
                + " "
                + without_controller[insertion:]
            )
            mutated = implementation.replace(method, reordered, 1)
            with self.subTest(version=version):
                with self.assertRaisesRegex(
                    SystemExit, f"Flow{version} accept_reconciliation"
                ):
                    audit.foundation_reconnect_documents(task, mutated, verifier)

    @staticmethod
    def _foundation_documents() -> tuple[str, str, str]:
        return (
            audit.text(
                "docs/agents/tasks/archive/OTV2-20260826-impl-foundation-reconnect-durability.md"
            ),
            audit.text("apps/game-server/src/foundation/admission_recovery_inner.rs"),
            audit.text("apps/game-server/src/foundation/fnd04_verifier.rs"),
        )

    @staticmethod
    def _replace_compact_fragment(block: str, fragment: str) -> str:
        pattern = re.sub(r"\\ ", r"\\s+", re.escape(fragment))
        mutated, replacements = re.subn(pattern, "true", block, count=1)
        if replacements != 1:
            raise AssertionError(f"expected exactly one match for {fragment!r}")
        return mutated


if __name__ == "__main__":
    unittest.main()
