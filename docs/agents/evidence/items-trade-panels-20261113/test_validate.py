"""Adversarial checks for evidence boundaries, without private inputs."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("trade_evidence_validator", HERE / "validate.py")
validator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validator)


class EvidenceBoundaries(unittest.TestCase):
    def setUp(self):
        self.food = json.loads((HERE / "food-initial-panels.json").read_text())
        self.decoration = json.loads((HERE / "decoration-initial-panels.json").read_text())

    def test_bare_price_cannot_gain_currency(self):
        quote = next(q for panel in self.food["items"][7]["panels"].values()
                     for q in panel["visible_entries"])
        self.assertEqual("OBSERVED_EXPLICIT", quote["currency_state"])
        quote["price_literal"] = "2"
        quote.update(currency_state="OBSERVED_EXPLICIT", currency_literal="gp")
        with self.assertRaisesRegex(ValueError, "currency inferred"):
            validator.validate_document(self.food)

    def test_genuine_unknown_is_supported_without_inventing_currency(self):
        self.assertEqual(("UNKNOWN", None), validator.currency("2"))
        self.assertEqual(("OBSERVED_EXPLICIT", "Event Points"),
                         validator.currency("70 x Event Points"))

    def test_material_native_corrections_are_required(self):
        receipts = json.loads((HERE / "source-receipts.json").read_text())
        backlog = json.loads((HERE / "retake-backlog.json").read_text())
        pairs = {(r["category"], r["physical_ui_ordinal"], r["panel"])
                 for r in backlog["retakes"]}
        for ordinal, panel, row, field, stale in [
            (76, "Buy From", 1, "npc_literal", "Khamna"),
            (255, "Buy From", 1, "price_literal", "250 x Christmas Token"),
            (382, "Sell To", 1, "npc_literal", "Shniel"),
        ]:
            with self.subTest(ordinal=ordinal):
                doc = copy.deepcopy(self.decoration)
                doc["items"][ordinal - 1]["panels"][panel]["visible_entries"][row - 1][field] = stale
                with self.assertRaisesRegex(ValueError, "native correction missing"):
                    validator.validate_material(HERE, receipts,
                                                {"Decoration": doc, "Food": self.food}, pairs)

    def test_initial_fourth_location_stays_unknown_after_full_retake(self):
        initial = self.decoration["items"][45]["panels"]["Buy From"]["visible_entries"]
        full = json.loads((HERE / "decoration-full-panel-46-buy-from.json").read_text())
        self.assertEqual("UNKNOWN", initial[3]["location_state"])
        self.assertEqual("Cormaya", full["rows"][3]["quote"]["location_literal"])
        self.assertEqual(18, len(full["rows"]))
        self.assertEqual(initial[:3], [r["quote"] for r in full["rows"][:3]])

    def test_private_narrative_injection_is_rejected(self):
        self.food["items"][0]["Description"] = "Private text must stay outside this export"
        with self.assertRaisesRegex(ValueError, "unexpected or missing fields"):
            validator.validate_document(self.food)

    def test_same_name_rows_cannot_replace_ordinals(self):
        self.decoration["items"][388] = copy.deepcopy(self.decoration["items"][387])
        with self.assertRaisesRegex(ValueError, "ordinal/category"):
            validator.validate_document(self.decoration)

    def test_omitting_correction_is_rejected(self):
        entry = self.decoration["items"][309]["panels"]["Sell To"]["visible_entries"][0]
        entry.update(npc_literal="Tasir", first_line_literal="10 gp, Tasir")
        with self.assertRaisesRegex(ValueError, "Yasir overlay"):
            validator.validate_document(self.decoration)

    def test_global_npc_completion_cannot_be_claimed(self):
        self.food["coverage"]["full_npc_lists"] = "COMPLETE"
        with self.assertRaisesRegex(ValueError, "distinguish initial"):
            validator.validate_document(self.food)


if __name__ == "__main__":
    unittest.main()
