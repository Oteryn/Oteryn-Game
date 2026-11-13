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
        cls.names = json.loads((HERE / "item-names-description-custody.json").read_text())
        cls.aliases = json.loads((HERE / "alias-taxonomy-matrix.json").read_text())
        cls.static = json.loads((HERE / "static-dataset-readiness.json").read_text())
        cls.weapons = json.loads((HERE / "weapon-comparison-supplement.json").read_text())
        cls.whole_names = json.loads((HERE / "whole-cohort-name-candidates.json").read_text())
        cls.whole_desc = json.loads((HERE / "whole-cohort-description-status.json").read_text())
        cls.desc_summary = json.loads((HERE / "whole-cohort-description-summary.json").read_text())
        cls.whole_npc = json.loads((HERE / "whole-cohort-npc-axes.json").read_text())
        cls.npc_summary = json.loads((HERE / "whole-cohort-npc-summary.json").read_text())
        cls.joins = validator.validate_whole_names(cls.whole_names, cls.names, cls.coverage)

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

    def test_exact_item_name_cannot_be_normalized(self):
        doc = copy.deepcopy(self.names)
        column = doc["columns"][doc["field_order"].index("name_exact_utf8")]
        column["values"][0] += " "
        with self.assertRaisesRegex(ValueError, "exact UTF8 name"):
            validator.validate_names(doc, self.coverage)

    def test_unknown_encoding_cannot_receive_zero_line_count(self):
        doc = copy.deepcopy(self.names)
        rows = validator.decode_names(doc)
        row = next(i for i, r in enumerate(rows) if r["description_state"] ==
                   "FIELD_PRESENT_ENCODING_UNKNOWN")
        column = doc["columns"][doc["field_order"].index("description_line_count")]
        column["dictionary"].append(0)
        column["indices"][row] = len(column["dictionary"]) - 1
        with self.assertRaisesRegex(ValueError, "unknown encoding"):
            validator.validate_names(doc, self.coverage)

    def test_alias_npc_count_must_match_source_cohort(self):
        doc = copy.deepcopy(self.aliases)
        doc["aliases"][0]["members"][0]["NPC_raw_tuple_occurrences"] += 1
        with self.assertRaisesRegex(ValueError, "alias market/NPC source join"):
            validator.validate_alias_matrix(doc, self.names, self.coverage)

    def test_raw_static_counts_cannot_admit_runtime_profile(self):
        doc = copy.deepcopy(self.static)
        doc["profile_admission"]["runtime_admitted"] = True
        with self.assertRaisesRegex(ValueError, "must not gain admission"):
            validator.validate_static(doc)

    def test_weapon_formatter_cannot_be_replaced_by_retail_label(self):
        doc = copy.deepcopy(self.weapons)
        field = next(f for f in doc["rows"][0]["layers"]["canonical"]["fields"]
                     if f["field"] == "Weapon Type")
        field["source_formatter_literal"] = field["observed_literal"]
        field["status"] = "LITERAL_EQUAL_CANDIDATE_ONLY"
        with self.assertRaisesRegex(ValueError, "exact bounded source projection"):
            validator.validate_weapons(doc)

    def test_whole_cohort_cannot_drop_nonmarket_appearance(self):
        doc = copy.deepcopy(self.whole_names)
        absent = {row["appearance_object_id"] for row in validator.decode_table(
            self.coverage["record_columns"], self.coverage["record_arrays"])
            if row["market_category"] is None}
        index = next(i for i, row in enumerate(doc["rows"]) if row[0] in absent)
        doc["rows"].pop(index)
        with self.assertRaisesRegex(ValueError, "whole name row identity"):
            validator.validate_whole_names(doc, self.names, self.coverage)

    def test_description_cannot_drop_ambiguous_candidate(self):
        doc = copy.deepcopy(self.whole_desc)
        row = next(row for row in doc["rows"] if len(row[2]) > 1)
        row[2].pop()
        row[3].pop()
        with self.assertRaisesRegex(ValueError, "Description candidate alignment"):
            validator.validate_whole_descriptions(doc, self.desc_summary, self.joins, self.names)

    def test_description_unknown_encoding_cannot_claim_equality(self):
        doc = copy.deepcopy(self.whole_desc)
        row = next(row for row in doc["rows"] if
                   doc["client_description_state_codes"][row[1]] ==
                   "FIELD_PRESENT_ENCODING_UNKNOWN" and row[2])
        row[2][0] = doc["comparison_result_codes"].index(
            "EXACT_UTF8_BYTES_AND_TEXT_EQUAL_CANDIDATE_ONLY")
        with self.assertRaisesRegex(ValueError, "unknown encoding acquired text equality"):
            validator.validate_whole_descriptions(doc, self.desc_summary, self.joins, self.names)

    def test_npc_zero_price_cannot_disappear(self):
        doc = copy.deepcopy(self.whole_npc)
        prices = doc["tuple_columns"]["sale_price_raw"]
        prices[prices.index(0)] = 1
        with self.assertRaisesRegex(ValueError, "raw NPC price/zero changed"):
            validator.validate_whole_npc(doc, self.npc_summary, self.joins,
                                         validator.decode_columns(self.npc))

    def test_npc_second_price_axis_cannot_be_dropped(self):
        doc = copy.deepcopy(self.whole_npc)
        doc["axis_columns"]["canonical"]["status_code"].pop()
        with self.assertRaisesRegex(ValueError, "NPC both-axis coverage"):
            validator.validate_whole_npc(doc, self.npc_summary, self.joins,
                                         validator.decode_columns(self.npc))


if __name__ == "__main__":
    unittest.main()
