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
        self.assertEqual("UNKNOWN", quote["currency_state"])
        quote.update(currency_state="OBSERVED_EXPLICIT", currency_literal="gp")
        with self.assertRaisesRegex(ValueError, "currency inferred"):
            validator.validate_document(self.food)

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
