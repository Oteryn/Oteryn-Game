"""Prevent evidence promotion, fabricated quotes and erased unknowns."""
import hashlib
import json
import unittest

import capture_bounds
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

    def test_public_recording_cannot_claim_controlled_gameplay(self):
        self.packet["research_limits"]["observations_performed"] = 1
        with self.assertRaisesRegex(ValueError, "no gameplay"):
            research_closure.validate(self.packet)

    def test_public_recording_requires_frame_receipts(self):
        self.packet["sources"]["minerva_public_scroll_use_2025"]["frames"] = []
        with self.assertRaisesRegex(ValueError, "frame receipts"):
            research_closure.validate(self.packet)

    def test_equipment_disagreements_cannot_be_missing_profiles(self):
        self.packet["equipment_source_disposition"]["counts"]["disputed_items_with_no_selected_profile"] = 101
        with self.assertRaisesRegex(ValueError, "missing selected profiles"):
            research_closure.validate(self.packet)

    def test_reported_armor_order_cannot_be_reversed(self):
        fact = next(f for f in self.packet["facts"] if f["id"] == "physical_armor_stage_current_reported_test_reference")
        fact["value"]["physical_stage"] = "ARMOR_BEFORE_RESISTANCE"
        with self.assertRaisesRegex(ValueError, "qualified completion"):
            research_closure.validate(self.packet)

    def test_historical_life_statement_cannot_certify_current_continuity(self):
        fact = next(f for f in self.packet["facts"] if f["id"] == "life_leech_overkill_reported_damage_basis_2024")
        fact["value"]["current_target_continuity"] = True
        with self.assertRaisesRegex(ValueError, "qualified completion"):
            research_closure.validate(self.packet)

    def test_vibrancy_example_cannot_become_future_attack_immunity(self):
        fact = next(f for f in self.packet["facts"] if f["id"] == "vibrancy_successful_pvp_retrigger_net_result")
        fact["value"]["future_attacks_always_deflected"] = True
        with self.assertRaisesRegex(ValueError, "qualified completion"):
            research_closure.validate(self.packet)

    def test_primary_strike_exceptions_cannot_be_erased(self):
        fact = next(f for f in self.packet["facts"] if f["id"] == "wand_rod_strike_current_primary_threshold_and_exceptions")
        fact["value"]["named_exceptions"].remove("Deepling Fork")
        with self.assertRaisesRegex(ValueError, "qualified completion"):
            research_closure.validate(self.packet)

    def test_source_choice_matrix_cannot_change_selected_types(self):
        self.packet["equipment_source_disposition"]["current_source_choice_matrix"]["items"][0]["selected_allowed_types"] = {}
        with self.assertRaisesRegex(ValueError, "matrix selection"):
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

    def _set_excerpt(self, source_id, text):
        source = self.packet["sources"][source_id]
        source["captured_text"] = text
        source["captured_text_sha256"] = hashlib.sha256(text.encode()).hexdigest()

    def test_full_page_capture_is_rejected(self):
        source = self.packet["sources"]["qa24514"]
        self._set_excerpt("qa24514", capture_bounds.EXCERPT_SEPARATOR.join(
            [source["captured_text"]] + ["x" * 400] * 3))
        with self.assertRaisesRegex(ValueError, "exceeds its source bound"):
            research_closure.validate(self.packet)

    def test_overlong_passage_is_rejected(self):
        source = self.packet["sources"]["qa24514"]
        self._set_excerpt("qa24514", source["captured_text"] + " " + "x" * 450)
        with self.assertRaisesRegex(ValueError, "passage exceeds"):
            research_closure.validate(self.packet)

    def test_unbounded_capture_scope_is_rejected(self):
        self.packet["sources"]["qa24514"]["captured_text_scope"] = "Complete captured public source text"
        with self.assertRaisesRegex(ValueError, "bounded quoted excerpt"):
            research_closure.validate(self.packet)

    def test_excerpt_digest_mismatch_is_rejected(self):
        self.packet["sources"]["qa24514"]["captured_text"] += " extra"
        with self.assertRaisesRegex(ValueError, "digest mismatch"):
            research_closure.validate(self.packet)


if __name__ == "__main__":
    unittest.main()
