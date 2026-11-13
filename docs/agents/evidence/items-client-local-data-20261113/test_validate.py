"""Negative checks for lossy decoding and evidence-boundary violations."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("local_data_validator", HERE / "validate.py")
validator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validator)


class EvidenceBoundaries(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.npc = json.loads((HERE / "client-npc-tuples.json").read_text())
        cls.coverage = json.loads((HERE / "client-cyclopedia-coverage.json").read_text())
        cls.candidates = json.loads((HERE / "field-comparison-candidates.json").read_text())
        cls.corrections = json.loads((HERE / "qualified-correction-deltas.json").read_text())

    def test_dictionary_index_must_not_wrap_negative(self):
        doc = copy.deepcopy(self.npc)
        column = next(c for c in doc["columns"] if c["encoding"] == "DICTIONARY")
        column["indices"][0] = -1
        with self.assertRaisesRegex(ValueError, "dictionary index"):
            validator.decode_columns(doc)

    def test_duplicate_tuple_ancestry_is_rejected(self):
        doc = copy.deepcopy(self.npc)
        for column in doc["columns"][:3]:
            column["values"][1] = column["values"][0]
        with self.assertRaisesRegex(ValueError, "ancestry deduplication"):
            validator.validate_npc(doc)

    def test_known_zero_cannot_be_replaced(self):
        doc = copy.deepcopy(self.npc)
        values = doc["columns"][doc["field_order"].index("sale_price_raw")]["values"]
        values[values.index(0)] = 1
        with self.assertRaisesRegex(ValueError, "known numeric zeros"):
            validator.validate_npc(doc)

    def test_absent_currency_does_not_gain_gp(self):
        doc = copy.deepcopy(self.npc)
        column = doc["columns"][doc["field_order"].index("currency_object_ref")]
        absent = next(x for x in column["dictionary"] if x["state"] == "FIELD_ABSENT")
        absent["value"] = "gp"
        with self.assertRaisesRegex(ValueError, "absent currency"):
            validator.validate_npc(doc)

    def test_private_description_and_absolute_path_are_rejected(self):
        for value in ({"Description": "synthetic private narrative"}, {"key": "/private/example"}):
            with self.subTest(value=value), self.assertRaises(ValueError):
                validator.privacy(value)

    def test_alias_group_cannot_drop_second_appearance(self):
        doc = copy.deepcopy(self.coverage)
        index = doc["alias_columns"].index("appearance_object_ids")
        doc["alias_arrays"][0][index].pop()
        with self.assertRaisesRegex(ValueError, "aliases must preserve"):
            validator.validate_coverage(doc)

    def test_npc_global_and_cyclopedia_subset_cannot_be_conflated(self):
        records = validator.decode_table(self.coverage["record_columns"],
                                         self.coverage["record_arrays"])
        ids = {row["appearance_object_id"] for row in records}
        rows = validator.decode_columns(self.npc)
        row = next(row for row in rows if row["appearance_object_id"] in ids)
        row["appearance_object_id"] = max(ids) + 1
        with self.assertRaisesRegex(ValueError, "populations must remain separate"):
            validator.validate_populations(ids, rows)

    def test_name_candidate_cannot_become_identity(self):
        doc = copy.deepcopy(self.candidates)
        doc["rows"][0]["identity_status"] = "PROVEN_SERVER_ID"
        with self.assertRaisesRegex(ValueError, "identity promotion"):
            validator.validate_candidates(doc)

    def test_rejected_price_cannot_enter_correction_delta(self):
        doc = copy.deepcopy(self.corrections)
        row = next(x for x in doc["records"] if x["category"] == "Decoration")
        row["physical_ui_ordinal"] = 116
        with self.assertRaisesRegex(ValueError, "rejected Decoration116"):
            validator.validate_corrections(doc)


if __name__ == "__main__":
    unittest.main()
