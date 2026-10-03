"""Closed source flavor guards and independent current siblings."""

import copy
import json
import unittest

import lower_item_description_packet as lane


class Descriptions(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.proof = json.loads((lane.ROOT / lane.PROOF).read_text())
        cls.historical = json.loads(
            (lane.ROOT / cls.proof["historical_source"]["path"]).read_text()
        )
        cls.definitions, cls.owners, _, cls.indexed, cls.pages = lane.names.load_inputs(
            lane.ROOT, cls.proof
        )

    def fixture(self):
        source = copy.deepcopy(self.proof["records"][0])
        old = copy.deepcopy(self.historical["records"][source["source_row_ordinal"]])
        definition = copy.deepcopy(self.definitions[source["target"]["key"]])
        params = lane.names.witness_params(
            source, self.indexed, self.pages, self.proof["qualification_cutoff"]
        )
        return source, old, definition, params

    def test_full114_real_source_packet_and_two_admitted_headers(self):
        packet = lane.build()
        self.assertEqual(packet["counts"], {"items": 114, "fields": 114})
        self.assertEqual(
            {
                r["target"]["key"]
                for r in packet["promotions"]
                if r["headers"]["materializable"]
            },
            {"oteryn:item.tibia.i237", "oteryn:item.tibia.i239"},
        )
        self.assertEqual(
            (
                lane.ROOT
                / "tools/content-schema/item-authoring/source_field_catalogs.py"
            ).read_bytes(),
            lane.base.checked(
                lane.ROOT,
                "tools/content-schema/item-authoring/source_field_catalogs.py",
                self.proof["input_digests"][
                    "tools/content-schema/item-authoring/source_field_catalogs.py"
                ],
            ),
        )

    def test_name_header_leaf_and_source_opposition_fail_closed(self):
        for operation in range(7):
            s, old, d, params = self.fixture()
            if operation == 0:
                d["materializable"] = 0
            elif operation == 1:
                d["semantics"]["presentation"]["value"]["name"]["value"] = "Other name"
            elif operation == 2:
                d["semantics"]["presentation"]["value"]["description"] = {
                    "state": "CONFLICT"
                }
            elif operation == 3:
                params[0]["flavortext"] = [s["description"], s["description"]]
            elif operation == 4:
                params[0]["flavortext"] = [""]
            elif operation == 5:
                old["xml_description_witnesses"][0]["description_values"] = [
                    "Different XML description"
                ]
            else:
                d["identity"]["revision"] = "different"
            with self.assertRaises(ValueError):
                lane.qualify(s, old, d, self.owners, params)

    def test_unrelated_stack_leaf_is_preserved_and_description_is_idempotent(self):
        s, old, d, params = self.fixture()
        d["semantics"]["stack"] = {
            "state": "KNOWN",
            "value": {
                "stackable": {"state": "KNOWN", "value": False},
                "stack_max": {"state": "UNKNOWN"},
            },
        }
        before = copy.deepcopy(d)
        row = lane.qualify(s, old, d, self.owners, params)
        self.assertEqual(d, before)
        d["semantics"]["presentation"]["value"]["description"] = {
            "state": "KNOWN",
            "value": s["description"],
        }
        self.assertEqual(row, lane.qualify(s, old, d, self.owners, params))
        d["semantics"]["presentation"]["value"]["description"]["value"] = (
            "GameOwned text"
        )
        with self.assertRaises(ValueError):
            lane.qualify(s, old, d, self.owners, params)

    def test_raw_own_identity_bytes_revision_and_whole_source_guards(self):
        for operation in range(4):
            s, _, _, _ = self.fixture()
            if operation == 0:
                s["sources"][0]["raw_infobox"] += "changed bytes"
            elif operation == 1:
                s["sources"][0]["revision_id"] += 1
            elif operation == 2:
                s["sources"] = []
            else:
                s["sources"][0]["inside_comment"] = True
            with self.assertRaises(ValueError):
                lane.names.witness_params(
                    s, self.indexed, self.pages, self.proof["qualification_cutoff"]
                )

    def test_utf8_bound_and_markup_are_literal_no_punctuation_normalization(self):
        self.assertEqual(lane.flavor("é" * 100), "é" * 100)
        self.assertEqual(lane.flavor("Ends with a period."), "Ends with a period.")
        for raw in (
            "é" * 101,
            "{{dynamic}}",
            "[[linked]]",
            "line\nbreak",
            "&amp;",
            "",
            " padded ",
        ):
            with self.assertRaises(ValueError):
                lane.flavor(raw)

    def test_pre_overlay_header_drift_still_fails_closed(self):
        # D316: the guard reads the pre-overlay stage; a real header drift there must fail.
        original = lane.pre_overlay_definitions
        key = json.loads(lane.OUTPUT.read_text())["promotions"][0]["target"]["key"]

        def drifted(root, current):
            definitions = original(root, current)
            row = definitions[key]
            definitions[key] = row | {"materializable": not row["materializable"]}
            return definitions

        lane.pre_overlay_definitions = drifted
        try:
            with self.assertRaisesRegex(ValueError, "header drift"):
                lane.build()
        finally:
            lane.pre_overlay_definitions = original


if __name__ == "__main__":
    unittest.main()
