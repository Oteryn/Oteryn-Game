"""Prevent evidence promotion, fabricated quotes and erased unknowns."""
import json
import unittest

import research_closure


class ResearchClosureTests(unittest.TestCase):
    def setUp(self):
        self.packet = json.loads(research_closure.PACKET.read_text())

    def test_delivered_public_facts_validate(self):
        research_closure.validate(self.packet)

    def test_blanket_global_certification_is_rejected(self):
        self.packet["full_global_parity_proven"] = True
        with self.assertRaisesRegex(ValueError, "full Global parity"):
            research_closure.validate(self.packet)

    def test_invented_scroll_consumption_is_rejected(self):
        group = next(g for g in self.packet["groups"] if g["id"] == "scroll_consumption")
        group["remaining_public_fields"][0]["value"] = 1
        with self.assertRaisesRegex(ValueError, "invented Global"):
            research_closure.validate(self.packet)

    def test_ots_reference_cannot_become_global_fact(self):
        next(iter(self.packet["sources"].values()))["source_role"] = "OTS_IMPLEMENTATION_REFERENCE"
        with self.assertRaisesRegex(ValueError, "OTS code"):
            research_closure.validate(self.packet)

    def test_unobserved_video_is_not_gameplay_evidence(self):
        self.packet["research_limits"]["observations_performed"] = 1
        with self.assertRaisesRegex(ValueError, "no gameplay"):
            research_closure.validate(self.packet)

    def test_fabricated_quote_is_rejected(self):
        self.packet["facts"][0]["quote_refs"][0]["text"] = "Every rejected scroll is always refunded."
        with self.assertRaisesRegex(ValueError, "quote is not present"):
            research_closure.validate(self.packet)

    def test_dropped_group_is_rejected(self):
        self.packet["groups"].pop()
        with self.assertRaisesRegex(ValueError, "each original"):
            research_closure.validate(self.packet)

    def test_fabricated_conflict_witness_is_rejected(self):
        self.packet["source_conflicts"][-1]["literal_quote"] = "The PZ timer always resets to zero."
        with self.assertRaisesRegex(ValueError, "conflict quote missing"):
            research_closure.validate(self.packet)


if __name__ == "__main__":
    unittest.main()
