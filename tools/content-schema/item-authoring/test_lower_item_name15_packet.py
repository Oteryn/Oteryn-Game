"""Closed official-only15 correction and independent current source guards."""

import copy
import json
import unittest

import lower_item_name15_packet as lane


class Names15(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.proof = json.loads((lane.ROOT / lane.PROOF).read_text())
        cls.inputs = lane.load_inputs(lane.ROOT, cls.proof)

    def fixture(self):
        source = copy.deepcopy(self.proof["records"][0])
        return source, copy.deepcopy(self.inputs[0][source["target"]["key"]])

    def test_actual15_complete_source_packet_and_repeated_numeric_names(self):
        packet = lane.build()
        self.assertEqual(packet["counts"], {"items": 15, "fields": 15})
        self.assertEqual(
            {int(r["target"]["key"].rsplit(".i", 1)[1]) for r in packet["promotions"]},
            set(lane.IDS),
        )
        self.assertEqual(
            [r["name"] for r in packet["promotions"]].count("blade of carving"), 2
        )
        self.assertEqual(
            [r["name"] for r in packet["promotions"]].count("wand of remedy"), 2
        )
        self.assertTrue(
            all(r["headers"]["materializable"] is False for r in packet["promotions"])
        )

    def test_only_exact_imported_literal_or_identical_name_not_another_generic(self):
        for name in [
            {"state": "UNKNOWN"},
            {"state": "CONFLICT"},
            {"state": "NOT_APPLICABLE"},
            {"state": "KNOWN", "value": "weapon of carving"},
            {"state": "KNOWN", "value": "GameOwned name"},
        ]:
            s, d = self.fixture()
            d["semantics"]["presentation"]["value"]["name"] = name
            with self.assertRaises(ValueError):
                lane.qualify(s, d, self.inputs[1])
        s, d = self.fixture()
        row = lane.qualify(s, d, self.inputs[1])
        d["semantics"]["presentation"]["value"]["name"] = {
            "state": "KNOWN",
            "value": s["name"],
        }
        self.assertEqual(lane.qualify(s, d, self.inputs[1]), row)

    def test_full_identity_headers_and_GameOwned_owner_fail_closed(self):
        for operation in range(6):
            s, d = self.fixture()
            owners = copy.deepcopy(self.inputs[1])
            if operation == 0:
                d["identity"]["revision"] = "definition-r2"
            elif operation == 1:
                d["materializable"] = 0
            elif operation == 2:
                d["stack_class"] = "Stackable"
            elif operation == 3:
                s["source_item_id"] = 23578
            elif operation == 4:
                owners.append(
                    {"item": s["target"], "presentation": {"name": "GameOwned"}}
                )
            else:
                s["previous_imported_name"] = "weapon of carving"
            with self.assertRaises(ValueError):
                lane.qualify(s, d, owners)

    def test_unrelated_description_stack_and_existing_other_owner_are_preserved(self):
        s, d = self.fixture()
        d["semantics"]["presentation"]["value"]["description"] = {
            "state": "KNOWN",
            "value": "Future literal.",
        }
        d["semantics"]["stack"] = {
            "state": "KNOWN",
            "value": {
                "stackable": {"state": "KNOWN", "value": False},
                "stack_max": {"state": "UNKNOWN"},
            },
        }
        owner = {"item": s["target"], "forge": {"classification": 2, "max_tier": 2}}
        before = copy.deepcopy(d)
        row = lane.qualify(s, d, [owner])
        self.assertEqual(d, before)
        self.assertEqual(row["name"], s["name"])

    def test_actual_official_duplicate_field_wrong_ID_and_invalid_UTF8_reject(self):
        s, _ = self.fixture()
        raw = self.inputs[6][s["source_item_id"]]
        name = s["name"].encode()
        field = b"\x22" + bytes([len(name)]) + name
        with self.assertRaises(ValueError):
            lane.official(raw + field, s)
        wrong = copy.deepcopy(s)
        wrong["source_item_id"] = 23578
        with self.assertRaises(ValueError):
            lane.official(raw, wrong)
        self.assertIn(field, raw)
        with self.assertRaises((ValueError, UnicodeDecodeError)):
            lane.official(
                raw.replace(
                    field, b"\x22" + bytes([len(name)]) + b"\xff" * len(name), 1
                ),
                s,
            )


if __name__ == "__main__":
    unittest.main()
