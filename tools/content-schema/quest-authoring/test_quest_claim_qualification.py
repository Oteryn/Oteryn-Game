"""Unit tests for the quest claim qualification audit."""

import copy
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location(
    "qualification", HERE / "check_quest_claim_qualification.py"
)
qualification = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(qualification)


class QualificationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.ledger = qualification.generated_ledger()
        self.path = Path(self.temp.name) / "ledger.json"

    def check(self, ledger=None, coverage=qualification.COVERAGE):
        self.path.write_text(json.dumps(ledger or self.ledger))
        return qualification.validate(self.path, coverage)[0]

    def test_checked_in_ledger_is_valid(self):
        errors, counts = qualification.validate()
        self.assertEqual(errors, [])
        # Keep the exact outstanding denominator; qualified and not-playable
        # source claims remain in their separate coverage categories.
        self.assertEqual(sum(counts.values()), 162)

    def test_missing_row_is_rejected(self):
        # Every still-pending row must remain in the ledger. The collected
        # ITEMHOLD claims now have real dispatch proof and leave this inventory.
        for key in self.ledger["claims"]:
            with self.subTest(claim=key):
                ledger = copy.deepcopy(self.ledger)
                del ledger["claims"][key]
                self.assertIn(f"missing claim: {key}", self.check(ledger))

    def test_itemhold_admissions_retain_their_qualified_source_witnesses(self):
        coverage = json.loads(qualification.COVERAGE.read_text())
        claims = {chest['claim']: chest for quest in coverage['quests']
                  for chest in quest['chests']}
        for key in qualification.ITEMHOLD_ADMISSIONS:
            with self.subTest(claim=key):
                self.assertNotIn(key, self.ledger['claims'])
                self.assertEqual(claims[key]['status'], 'dispatch_pg_qualified')
                self.assertEqual(claims[key]['reason'], {
                    'oteryn:reward-claim.kv.ornamentedshield':
                        'quest_catalogue_chests_exact_partial_charges_use_replay_and_refusal',
                    'oteryn:reward-claim.kv.powerring2':
                        'quest_catalogue_chests_full_charge_rewards_use_replay_and_refusal',
                }[key])

    def test_invented_test_is_rejected(self):
        record = next(iter(self.ledger["claims"].values()))
        record.update(disposition="test", dependency="QUEST-CALLBACKS-1", evidence={
            "kind": "test", "path": "does/not/exist.rs", "fully_qualified_name": "x::invented",
            "crate_or_target": "gameplay-dispatch", "observed_result": "minted once then refused",
            "asserted_scenario": "all required production-route assertions",
        })
        self.assertTrue(any("unresolved test" in error for error in self.check()))

    def test_denominator_change_is_rejected(self):
        coverage = json.loads(qualification.COVERAGE.read_text())
        coverage["counts"]["supported_unqualified"] = 240
        path = Path(self.temp.name) / "coverage.json"
        path.write_text(json.dumps(coverage))
        self.assertTrue(any("denominator drift" in error for error in self.check(coverage=path)))

    def test_stale_source_witness_is_rejected(self):
        self.ledger["source_revisions"]["canary"] = "stale"
        self.assertIn("stale pinned source witness", self.check())

    def test_invalid_callback_dependency_is_rejected(self):
        record = next(iter(self.ledger["claims"].values()))
        record["dependency"] = "QUEST-CALLBACKS-3"
        self.assertTrue(any("matching dependency" in error for error in self.check()))

    def test_duplicate_claim_key_is_rejected(self):
        text = json.dumps(self.ledger)
        key = next(iter(self.ledger["claims"]))
        marker = json.dumps(key) + ":"
        duplicate = text.replace(marker, marker + "{} , " + marker, 1)
        self.path.write_text(duplicate)
        errors, _ = qualification.validate(self.path)
        self.assertTrue(any("duplicate JSON key" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
