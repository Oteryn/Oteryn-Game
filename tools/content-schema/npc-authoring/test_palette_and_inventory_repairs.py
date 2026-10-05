"""Palette and conservative NPC identity regressions for the R5 completion."""

import copy
import json
import unittest
from pathlib import Path

import npc_inventory_reconcile as inventory
import validate_promotion
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parent


class PaletteAndInventoryRepairs(unittest.TestCase):
    def test_all_palette_slots_reject_outside_client_domain(self):
        schema = json.loads((ROOT / "npc.schema.json").read_text())
        validator = Draft202012Validator(schema)
        bundle = json.loads((ROOT / "samples/bundles/canary/sam.json").read_text())
        for slot in ("head", "body", "legs", "feet"):
            for value in (-1, 133, 1156, 2**32, True):
                with self.subTest(slot=slot, value=value):
                    invalid = copy.deepcopy(bundle)
                    invalid["definition"]["presentation"]["outfit"][slot] = value
                    self.assertTrue(list(validator.iter_errors(invalid)))
            for value in (0, 132):
                valid = copy.deepcopy(bundle)
                valid["definition"]["presentation"]["outfit"][slot] = value
                self.assertEqual(list(validator.iter_errors(valid)), [])

    def test_promotion_validator_keeps_native_palette_bound(self):
        report = json.loads((ROOT / "samples/promotion-candidates-v1.json").read_text())
        candidate = next(c for c in report["candidates"] if c["name"] == "Sam")
        for slot in ("head", "body", "legs", "feet"):
            invalid = copy.deepcopy(candidate)
            invalid["presentation"]["outfit"][slot] = 133
            self.assertTrue(validate_promotion.candidate_errors(invalid, 0))
        held = next(c for c in report["candidates"] if c["name"] == "Hagor")
        self.assertIsNone(held["presentation"]["outfit"])
        self.assertEqual(validate_promotion.candidate_errors(held, 0), [])
        proof = next(
            r for r in held["left_out"] if r["reason"] == "PALETTE_OUT_OF_RANGE"
        )
        self.assertEqual(proof["source_value"]["feet"], 1156)

    def observe(self, names, existing=None, removed=""):
        br = [{"name": name, "role": "npc", "removed": removed} for name in names]
        tp = [{"name": name} for name in names]
        return inventory.group_observations(br, tp, existing or {})

    def test_wiki_npc_suffix_is_an_existing_identity_alias(self):
        rows = self.observe(["Hyacinth (NPC)"], {"hyacinth": "oteryn:npc.hyacinth"})
        self.assertEqual(rows[0]["status"], "EXISTING_OBSERVED_IDENTITY")
        self.assertEqual(rows[0]["npc_key"], "oteryn:npc.hyacinth")

    def test_location_and_numeric_variants_do_not_create_duplicate_actor(self):
        rows = self.observe(
            ["A Dead Bureaucrat"],
            {
                "a dead bureaucrat (1)": "oteryn:npc.a_dead_bureaucrat_1",
                "a dead bureaucrat (2)": "oteryn:npc.a_dead_bureaucrat_2",
            },
        )
        self.assertEqual(rows[0]["status"], "AMBIGUOUS_VARIANT_IDENTITY_HOLD")
        self.assertEqual(len(rows[0]["alias_candidates"]), 2)

    def test_apostrophe_normalization_preserves_existing_identity(self):
        rows = self.observe(
            ["S’Zallar M’Andar"], {"s'zallar m'andar": "oteryn:npc.s_zallar_m_andar"}
        )
        self.assertEqual(rows[0]["status"], "EXISTING_OBSERVED_IDENTITY")

    def test_lossy_native_key_collision_is_held(self):
        rows = self.observe(["O-Reyn", "O Reyn"])
        self.assertTrue(all(r["status"] == "NATIVE_KEY_COLLISION_HOLD" for r in rows))

    def test_single_wiki_or_removed_identity_is_not_promoted(self):
        rows = inventory.group_observations([{"name": "Chip", "role": "npc"}], [], {})
        self.assertEqual(rows[0]["status"], "SINGLE_WIKI_IDENTITY_HOLD")
        self.assertEqual(
            self.observe(["Deseus"], removed="12.70")[0]["status"], "WIKI_REMOVAL_HOLD"
        )

    def test_empty_placeholder_and_ambiguous_wiki_identity_are_held(self):
        self.assertEqual(self.observe(["..."])[0]["status"], "EMPTY_IDENTITY_HOLD")
        rows = self.observe(["Chip", "Chip"])
        self.assertEqual(rows[0]["status"], "AMBIGUOUS_WIKI_IDENTITY_HOLD")

    def test_new_identity_is_source_only_not_appearance_or_movement_claim(self):
        row = self.observe(["Chip"])[0]
        self.assertEqual(row["status"], "TWO_WIKI_IDENTITY_PROPOSAL")
        self.assertEqual(row["npc_key"], "oteryn:npc.chip")
        self.assertNotIn("presentation", row)
        self.assertNotIn("behavior", row)


if __name__ == "__main__":
    unittest.main()
