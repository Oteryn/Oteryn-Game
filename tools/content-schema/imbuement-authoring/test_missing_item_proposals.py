"""Check proposed Items against the owning contract and admission boundary."""
from copy import deepcopy
import unittest
from unittest.mock import patch

import missing_item_proposals as proposals


class MissingItemProposalTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.packet = proposals.build()

    def test_both_proposals_pass_owning_validator_with_no_warnings(self):
        self.assertEqual(len(self.packet["proposals"]), 2)
        for row in self.packet["proposals"]:
            self.assertEqual(row["identity_state"], "PROPOSED_NOT_REGISTERED")
            self.assertEqual(row["validation"]["errors"], [])
            self.assertEqual(row["validation"]["warnings"], [])

    def test_sailor_is_after_july_target_and_bursa_preexisting(self):
        rows = {row["source_client_id"]: row for row in self.packet["proposals"]}
        self.assertEqual(rows[53192]["target_time_status"], "INTRODUCED_AFTER_TARGET")
        self.assertEqual(rows[53192]["source_facts"]["facts"]["introduced_on"], "2026-08-04")
        self.assertEqual(rows[49160]["authoring_definition"]["container"]["capacity"], 1)

    def test_raw_client_slot_count_corroborates_both_authored_items(self):
        for row in self.packet["proposals"]:
            self.assertEqual(row["primary_client"]["imbuement_slots"], 1)
            self.assertEqual(row["primary_client"]["imbuement_slots"],
                             row["authoring_definition"]["imbuement"]["slot_count"])

    def test_owning_contract_rejects_impossible_container_capacity(self):
        import validate_item
        row = deepcopy(self.packet["proposals"][0])
        row["authoring_definition"]["container"]["capacity"] = 0
        errors, _ = validate_item.validate(row["authoring_definition"], row["dependencies"])
        self.assertTrue(errors)

    def test_proposal_cannot_silently_overwrite_admitted_identity(self):
        with patch.object(proposals, "canonical_items", return_value={"oteryn:item.tibia.i49160": {}}):
            with self.assertRaisesRegex(ValueError, "already admitted"):
                proposals.build()

    def test_test_labels_do_not_assert_obtainability(self):
        self.assertEqual(len(self.packet["test_item_evidence"]), 4)
        for row in self.packet["test_item_evidence"]:
            self.assertIn("AVAILABILITY_UNPROVEN", row["classification"])
            self.assertTrue(row["wiki_observations"])
            self.assertTrue(row["primary_client"]["flags"]["flags.take"])


if __name__ == "__main__":
    unittest.main()
