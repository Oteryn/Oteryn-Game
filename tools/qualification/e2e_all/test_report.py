import unittest
from pathlib import Path
import re

from report import combat_results, map_results, matrix_rows, results, summary


class ReportTests(unittest.TestCase):
    def test_map_checks_include_failures_skips_and_reconciled_summaries(self):
        log = (
            "SEAM_EVIDENCE MAP_CHECK stairs_up 100,101,7 PASS matched\n"
            "SEAM_EVIDENCE MAP_CHECK stairs_up 102,101,7 FAIL wrong floor\n"
            "MAP_CHECK npc 103,101,7 SKIPPED no content id on wire\n"
            "SEAM_EVIDENCE MAP_SUMMARY kind=stairs_up pass=1 fail=1 skipped=0\n"
            "MAP_SUMMARY kind=npc pass=0 fail=0 skipped=1\n"
        )
        checks, totals = map_results(log)
        self.assertEqual(len(checks), 3)
        self.assertEqual(totals, {"stairs_up": [1, 1, 0], "npc": [0, 0, 1]})
        found = results("SEAM_EVIDENCE e2e_all scenario=M01.full status=PASS seconds=1")
        output, complete = summary([("M01", "Movement")], found, map_evidence=(checks, totals))
        self.assertIn("map stairs_up 102,101,7 | FAIL", output)
        self.assertIn("map npc 103,101,7 | SKIPPED", output)
        self.assertIn("map summary stairs_up | pass=1 fail=1 skipped=0", output)
        self.assertFalse(complete)

    def test_map_pass_does_not_promote_full_mechanic(self):
        log = "MAP_CHECK field 0,0,7 PASS matched\nMAP_SUMMARY kind=field pass=1 fail=0 skipped=0"
        self.assertFalse(summary([("M01", "Movement")], {}, map_evidence=map_results(log))[1])
        self.assertEqual(map_results("unrelated log"), ([], {}))

    def test_map_malformed_duplicate_missing_and_mismatched_counts_fail_closed(self):
        check = "MAP_CHECK field 1,2,7 PASS matched\n"
        total = "MAP_SUMMARY kind=field pass=1 fail=0 skipped=0"
        for log in [
            check, total, check + check + total, check + total + "\n" + total,
            check + total.replace("pass=1", "pass=2"),
            check.replace("1,2,7", "1,2,16") + total,
            check.replace("PASS", "UNKNOWN") + total,
            check.replace("matched", "") + total,
            check + total + " trailing",
            total + "\n" + check,
        ]:
            with self.subTest(log=log), self.assertRaises(ValueError):
                map_results(log)

    def test_real_matrix_contains_every_done_and_partial_audit_row(self):
        root = Path(__file__).resolve().parents[3]
        audit = (root / "docs/architecture/reviews/OTERYN_GAME_CRYSTAL_REAUDIT_2026-10-28.md").read_text()
        expected = []
        for line in audit.splitlines():
            match = re.match(r"^\| ([A-Z]\d\d[a-z]?) —", line)
            if match and line.split("|")[5].strip() in {"**DONE**", "**PARTIAL**"}:
                expected.append(match.group(1))
        rows = matrix_rows((root / "apps/game-server/src/qualification/SCENARIO_MATRIX.md").read_text())
        self.assertEqual([row_id for row_id, _ in rows], expected)
        _, complete = summary(rows, {})
        self.assertFalse(complete)

    def test_bounded_pass_and_legacy_skip_cannot_qualify_a_mechanic(self):
        evidence = results(
            "SEAM_EVIDENCE mechanic=store skipped=pinned_platform_has_no_store_api\n"
            "SEAM_EVIDENCE e2e_all scenario=M01.step-relogin status=PASS seconds=1.2\n"
        )
        output, complete = summary([("M01", "Movement")], evidence)
        self.assertFalse(complete)
        self.assertIn("Movement | FAIL", output)
        self.assertIn("slice M01.step-relogin | PASS", output)

    def test_process_failure_overrides_complete_scenario(self):
        evidence = results("SEAM_EVIDENCE e2e_all scenario=M01.full status=PASS seconds=1\n")
        for node, client in [(1, 0), (0, 1)]:
            self.assertFalse(summary([("M01", "Movement")], evidence, node, client)[1])
        self.assertTrue(summary([("M01", "Movement")], evidence)[1])

    def test_missing_content_or_server_needs_evidence_and_never_completes(self):
        for status in ["SKIPPED", "MISSING-SERVER"]:
            with self.assertRaises(ValueError):
                results(f"SEAM_EVIDENCE e2e_all scenario=M01.full status={status} seconds=0")
            source = "content:oteryn:item.tibia.i123" if status == "SKIPPED" else "source:apps/game-server/src/node.rs:12"
            evidence = results(f"SEAM_EVIDENCE e2e_all scenario=M01.full status={status} seconds=0 evidence={source}")
            self.assertFalse(summary([("M01", "Movement")], evidence)[1])

    def test_duplicate_unknown_and_malformed_evidence_fails(self):
        line = "SEAM_EVIDENCE e2e_all scenario=M01.full status=PASS seconds=0"
        with self.assertRaises(ValueError):
            results(line + "\n" + line)
        with self.assertRaises(ValueError):
            results(line + " trailing garbage")
        with self.assertRaises(ValueError):
            summary([("I01", "Items")], results(line))

    def test_combat_checks_require_matching_summary_and_complete_case_set(self):
        log = (
            "COMBAT_CHECK C17a haste-speed PASS speed_increased\n"
            "COMBAT_SUMMARY row=C17a pass=1 fail=0 skipped=0\n"
        )
        self.assertEqual(combat_results(log)["C17a.full"][0], "PASS")
        self.assertEqual(results(log)["C17a.full"][0], "PASS")
        self.assertFalse(summary([("C17a", "Haste")], results(log), 1, 0)[1])
        incomplete = log.replace("C17a", "C01").replace("haste-speed", "offensive-stand")
        self.assertEqual(results(incomplete)["C01.full"][0], "FAIL")

    def test_complete_multi_check_combat_row_qualifies_only_without_failures(self):
        cases = ["words-mana", "spell-cooldown", "group-cooldown", "pz-aggression"]
        log = "\n".join(f"COMBAT_CHECK C04 {case} PASS observed" for case in cases)
        log += "\nCOMBAT_SUMMARY row=C04 pass=4 fail=0 skipped=0"
        self.assertTrue(summary([("C04", "Spells")], results(log))[1])
        failed = log.replace("pz-aggression PASS observed", "pz-aggression FAIL mana_spent")
        failed = failed.replace("pass=4 fail=0", "pass=3 fail=1")
        output, complete = summary([("C04", "Spells")], results(failed))
        self.assertFalse(complete)
        self.assertIn("pz-aggression:FAIL:mana_spent", output)

    def test_combat_failures_and_content_skips_never_qualify(self):
        for status, counts, reason in [
            ("FAIL", "pass=0 fail=1 skipped=0", "speed_unchanged"),
            ("SKIPPED", "pass=0 fail=0 skipped=1", "content:canary:spell/haste"),
        ]:
            log = f"COMBAT_CHECK C17a haste-speed {status} {reason}\nCOMBAT_SUMMARY row=C17a {counts}"
            self.assertEqual(results(log)["C17a.full"][0], status)
            self.assertFalse(summary([("C17a", "Haste")], results(log))[1])

    def test_combat_evidence_rejects_truncation_duplicates_and_bad_counts(self):
        check = "COMBAT_CHECK C17a haste-speed PASS speed_increased"
        total = "COMBAT_SUMMARY row=C17a pass=1 fail=0 skipped=0"
        for log in [
            check, total, check + "\n" + check + "\n" + total,
            check + "\n" + total + "\n" + total,
            check + "\n" + total.replace("pass=1", "pass=2"),
            total + "\n" + check,
            check.replace("PASS speed_increased", "SKIPPED no_equipment") + "\n" + total,
            check.replace("C17a", "C99") + "\n" + total.replace("C17a", "C99"),
            check + " trailing\nCOMBAT_SUMMARY row=C17a pass=-1 fail=0 skipped=0",
        ]:
            with self.subTest(log=log), self.assertRaises(ValueError):
                results(log)
        with self.assertRaises(ValueError):
            results(check + "\n" + total + "\nSEAM_EVIDENCE e2e_all scenario=C17a.full status=PASS seconds=0")

    def test_empty_duplicate_or_unplanned_matrix_fails(self):
        valid = "| M01 | Movement | owner | `M01.full`: do everything |"
        self.assertEqual(matrix_rows(valid), [("M01", "Movement")])
        for invalid in ["", valid + "\n" + valid, "| M01 | Movement | no scenario |"]:
            with self.assertRaises(ValueError):
                matrix_rows(invalid)

    def test_missing_client_needs_evidence_and_never_qualifies(self):
        found = results(
            "SEAM_EVIDENCE e2e_all scenario=P19.client status=MISSING-CLIENT seconds=0.000 evidence=no-reducer"
        )
        self.assertEqual(found["P19.client"][0], "MISSING-CLIENT")
        self.assertFalse(summary([("P19", "Outfits")], found)[1])
        with self.assertRaises(ValueError):
            results("SEAM_EVIDENCE e2e_all scenario=P19.client status=MISSING-CLIENT seconds=0")


if __name__ == "__main__":
    unittest.main()
