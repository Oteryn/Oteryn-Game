from __future__ import annotations

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


if __name__ == "__main__":
    unittest.main()
