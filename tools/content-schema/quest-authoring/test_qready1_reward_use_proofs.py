"""Qready1 proof import requires every real, successful, non-skipped PG case."""

import json
import re
import tempfile
import unittest
from pathlib import Path

import qready1_reward_use_proofs as lane
import quest_completion_matrix as matrix
import quest_reward_use_proofs as proofs
from quest_completion_matrix import load_reward_claims

ROOT = Path(__file__).resolve().parents[3]


class Qready1RewardUseProofTest(unittest.TestCase):
    def setUp(self):
        source = (ROOT / lane.TEST).read_text()
        self.markers = re.findall(
            r"POSTGRES_CASE_COMPLETED (qready1_\w+) duration=", source
        )
        self.log = (
            "\n".join(
                f"POSTGRES_CASE_COMPLETED {marker} duration=1.25s success=true"
                for marker in self.markers
            )
            + "\ntest result: ok. 17 passed; 0 failed;\n"
        )

    def test_complete_log_qualifies_every_owned_claim_in_existing_proof_loader(self):
        proof = lane.generate(ROOT, self.log)
        bindings = json.loads((ROOT / lane.BINDINGS).read_text())["bindings"]
        expected = {row["claim"] for row in bindings}
        self.assertEqual(expected, {row["claim_key"] for row in proof["cases"]})
        self.assertEqual(17, len({row["test"] for row in proof["cases"]}))
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            test = root / lane.TEST
            test.parent.mkdir(parents=True)
            test.write_bytes((ROOT / lane.TEST).read_bytes())
            path = root / proofs.PROOFS / "qready1.json"
            path.parent.mkdir(parents=True)
            path.write_text(json.dumps(proof))
            self.assertEqual(expected, proofs.load(root, load_reward_claims(ROOT)))

    def test_committed_pg_proof_promotes_all_17_owned_quests(self):
        proof = json.loads((ROOT / proofs.PROOFS / "qready1.json").read_text())
        expected = {
            row["claim"]
            for row in json.loads((ROOT / lane.BINDINGS).read_text())["bindings"]
        }
        self.assertEqual(expected, {row["claim_key"] for row in proof["cases"]})
        titles = {row["wiki_title"] for row in proof["cases"]}
        self.assertEqual(17, len(titles))
        rows = [
            row
            for row in matrix.expected(ROOT)["records"]
            if row["wiki_title"] in titles
        ]
        self.assertEqual(17, len(rows))
        self.assertTrue(
            all(
                row["implementation_state"] == "REWARD_CLAIM_USE_VERIFIED"
                for row in rows
            )
        )

    def test_skipped_duration_is_rejected(self):
        with self.assertRaises(ValueError):
            lane.generate(
                ROOT, self.log.replace("duration=1.25s", "duration=0.001s", 1)
            )

    def test_missing_marker_is_rejected(self):
        with self.assertRaises(ValueError):
            lane.generate(ROOT, self.log.replace(self.markers[0], "MISSING", 1))

    def test_duplicate_marker_is_rejected(self):
        with self.assertRaises(ValueError):
            lane.generate(ROOT, self.log + self.log.splitlines()[0])

    def test_failed_case_is_rejected_even_with_an_inconsistent_passing_summary(self):
        with self.assertRaises(ValueError):
            lane.generate(ROOT, self.log.replace("success=true", "success=false", 1))

    def test_partial_or_failed_run_is_rejected(self):
        for summary in ("16 passed; 0 failed;", "17 passed; 1 failed;"):
            with self.subTest(summary=summary), self.assertRaises(ValueError):
                lane.generate(ROOT, self.log.replace("17 passed; 0 failed;", summary))

    def test_focused_refresh_preserves_unselected_evidence(self):
        previous = lane.generate(ROOT, self.log)
        selected = "qready1_circle_room_quest_exact_reward_once"
        log = ("POSTGRES_CASE_COMPLETED qready1_circle_room_quest "
               "duration=2.5s success=true\n"
               "test result: ok. 1 passed; 0 failed;\n")
        refreshed = lane.generate(ROOT, log, selected={selected}, previous=previous)
        for bad_log in (log.replace('success=true', 'success=false'),
                        log.replace('duration=2.5s', 'duration=0.001s'),
                        log.replace('1 passed', '0 passed'),
                        log.replace('qready1_circle_room_quest', 'missing_marker')):
            with self.subTest(log=bad_log), self.assertRaises(ValueError):
                lane.generate(ROOT, bad_log, selected={selected}, previous=previous)
        for before, after in zip(previous["cases"], refreshed["cases"]):
            if before["test"] == selected:
                self.assertEqual(after["duration_seconds"], 2.5)
            else:
                self.assertEqual(before, after)
        previous["cases"][0]["claim_sha256"] = "stale"
        with self.assertRaises(ValueError):
            lane.generate(ROOT, log, selected={selected}, previous=previous)
        previous['test_sha256'] = 'changed'
        with self.assertRaises(ValueError):
            lane.generate(ROOT, log, selected={selected}, previous=previous)
