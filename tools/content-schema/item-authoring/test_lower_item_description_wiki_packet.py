"""Exact1515 Wiki-primary policy, disjoint scope, and sibling compatibility."""

import copy
import json
import unittest

import lower_item_description_wiki_packet as lane


class WikiDescriptions(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.proof = json.loads((lane.ROOT / lane.PROOF).read_text())
        h = cls.proof["historical_source"]
        cls.historical = json.loads((lane.ROOT / h["path"]).read_text())
        cls.definitions, cls.owners, _, cls.indexed, cls.pages = (
            lane.strict.names.load_inputs(lane.ROOT, cls.proof)
        )

    def fixture(self):
        s = copy.deepcopy(self.proof["records"][0])
        old = copy.deepcopy(self.historical["records"][s["source_row_ordinal"]])
        d = copy.deepcopy(self.definitions[s["target"]["key"]])
        params = lane.strict.names.witness_params(
            s, self.indexed, self.pages, self.proof["qualification_cutoff"]
        )
        return s, old, d, params

    def test_full_cohorts_disjoint1627_keep_periods_and_existing_materializability(
        self,
    ):
        strict, wiki = lane.strict.build(), lane.build()
        strict_ids = {r["target"]["key"] for r in strict["promotions"]}
        wiki_ids = {r["target"]["key"] for r in wiki["promotions"]}
        self.assertFalse(strict_ids & wiki_ids)
        self.assertEqual(len(strict_ids | wiki_ids), 1627)
        self.assertEqual(
            sum(r["headers"]["materializable"] for r in wiki["promotions"]), 10
        )
        # D289: i901 keeps its accepted Native core hold; D310: i36586 keeps main's sealed
        # Native state. Their literals are held, not promoted.
        self.assertEqual(
            [(h["target"]["key"], h["reason"]) for h in wiki["holds"]],
            [
                ("oteryn:item.tibia.i901", "D289_ACCEPTED_NATIVE_CORE_HOLD"),
                ("oteryn:item.tibia.i36586", "D310_MAIN_SEALED_NATIVE_STATE"),
            ],
        )
        self.assertFalse(
            {"oteryn:item.tibia.i901", "oteryn:item.tibia.i36586"} & wiki_ids
        )
        for row in wiki["promotions"]:
            self.assertTrue(row["description"].endswith("."))

    def test_no_punctuation_normalization_no_xml_agreement_or_other_difference_admission(
        self,
    ):
        for operation in range(5):
            s, old, d, params = self.fixture()
            if operation == 0:
                s["description"] = s["description"][:-1]
            elif operation == 1:
                old["xml_description_witnesses"][0]["description_values"] = [
                    s["description"]
                ]
            elif operation == 2:
                old["xml_description_witnesses"][0]["description_values"] = [
                    "Other XML text"
                ]
            elif operation == 3:
                params[0]["flavortext"] = [s["description"][:-1]]
            else:
                params[0]["flavortext"] = [s["description"], s["description"]]
            with self.assertRaises(ValueError):
                lane.qualify(s, old, d, self.owners, params)

    def test_current_headers_known_name_leaf_guard_and_unrelated_stack_successor(self):
        s, old, d, params = self.fixture()
        d["semantics"]["stack"] = {"state": "CONFLICT"}
        before = copy.deepcopy(d)
        row = lane.qualify(s, old, d, self.owners, params)
        self.assertEqual(d, before)
        d["semantics"]["presentation"]["value"]["description"] = {
            "state": "KNOWN",
            "value": s["description"],
        }
        self.assertEqual(row, lane.qualify(s, old, d, self.owners, params))
        for state in (
            {"state": "CONFLICT"},
            {"state": "KNOWN", "value": "GameOwned text"},
        ):
            changed = copy.deepcopy(d)
            changed["semantics"]["presentation"]["value"]["description"] = state
            with self.assertRaises(ValueError):
                lane.qualify(s, old, changed, self.owners, params)
        d["materializable"] = not d["materializable"]
        with self.assertRaises(ValueError):
            lane.qualify(s, old, d, self.owners, params)

    def test_pre_overlay_header_drift_still_fails_closed(self):
        # D316: the guard reads the pre-overlay stage; a real header drift there must fail.
        original = lane.strict.pre_overlay_definitions
        key = json.loads(lane.OUTPUT.read_text())["promotions"][0]["target"]["key"]

        def drifted(root, current):
            definitions = original(root, current)
            row = definitions[key]
            definitions[key] = row | {"materializable": not row["materializable"]}
            return definitions

        lane.strict.pre_overlay_definitions = drifted
        try:
            with self.assertRaisesRegex(ValueError, "header drift"):
                lane.build()
        finally:
            lane.strict.pre_overlay_definitions = original


if __name__ == "__main__":
    unittest.main()
