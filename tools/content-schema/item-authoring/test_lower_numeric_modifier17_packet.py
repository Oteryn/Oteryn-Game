"""Whole-vector source opposition, numeric units and current scoped guards."""

import copy
import json
import unittest

import lower_numeric_modifier17_packet as c


class NumericTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        receipt = json.loads((c.ROOT / c.RECEIPT).read_bytes())
        cls.entries = {
            r["item_id"]: r
            for p in receipt["source_proofs"]
            for r in json.loads((c.ROOT / p["path"]).read_bytes())["records"]
        }
        cls.indexed, cls.pages = c.frame.own_index(
            json.loads((c.ROOT / receipt["global_own_id_frame"]["path"]).read_bytes())
        )
        cls.defs = {
            r["definition"]["identity"]["key"]: r["definition"]
            for path in json.loads((c.ROOT / "content/items/index.json").read_bytes())[
                "shards"
            ]
            for r in json.loads((c.ROOT / path).read_bytes())["records"]
        }
        _, docs = c.membership.load_admitted(
            c.ROOT / "imports/official/appearance-membership"
        )
        cls.members = {
            label: {r[0]: r for r in doc["entries"]} for label, doc in docs.items()
        }
        cls.order = c.shared.native_order()

    def args(self, iid=36656):
        e = copy.deepcopy(self.entries[iid])
        official = e["official_source"]
        obj = {
            "id": iid,
            "name": official["official_name"],
            "flags": copy.deepcopy(official["official_flags"]),
            "object_sha256": official["official_raw_object_sha256"],
        }
        return [
            e,
            copy.deepcopy(self.defs[e["item_key"]]),
            copy.deepcopy(official["binding"]),
            obj,
            self.members,
            set(),
            e["source_whole_vector"]["value"],
        ]

    def test_actual_source17_explicit13_plus4_and_all_units(self):
        packet = c.build()
        self.assertEqual(packet["counts"], {"items": 17, "vectors": 17, "atoms": 50})
        self.assertEqual({r["source_item_id"] for r in packet["promotions"]}, c.IDS)
        self.assertEqual(
            sum(
                len(r["modifiers"])
                for r in packet["promotions"]
                if r["source_item_id"] in c.IDS13
            ),
            34,
        )
        for e in self.entries.values():
            self.assertEqual(
                c.source_vector(
                    e, self.indexed, self.pages, "2026-09-27T23:59:59Z", self.order
                ),
                e["source_whole_vector"]["value"],
            )
        percent = next(
            a
            for a in self.entries[36672]["source_whole_vector"]["value"]
            if a["kind"] == "MAGIC_SHIELD_CAPACITY_PERCENT"
        )
        self.assertEqual(
            percent["parameter"]["value"]["value"], {"numerator": 8, "denominator": 1}
        )

    def test_whole_presence_bounds_duplicates_and_no_reflection_element(self):
        for raw in [
            "42 Damage Reflection, 13 Damage Reflection",
            "42 Damage Reflection, unknown +1",
            "perfect shot +20 at range 65536",
            "2147483648 Damage Reflection",
            "cleave 101%",
            "magic shield capacity +80 and 101%",
            "magic shield capacity +2147483648 and 8%",
            "magic shield capacity +80/8%",
            "42 Damage Reflection,",
        ]:
            self.assertIsNone(c.whole_vector({"attrib": [raw]}, self.order), raw)
        for extra in [
            {"mantra": [""]},
            {"crithit_ch": [""]},
            {"hpleech_am": ["1%", "2%"]},
            {"elementalbond": ["physical"]},
        ]:
            self.assertIsNone(
                c.whole_vector({"attrib": ["42 Damage Reflection"]} | extra, self.order)
            )
        reflection = c.whole_vector({"attrib": ["42 Damage Reflection"]}, self.order)[
            "value"
        ][0]
        self.assertEqual(
            reflection["parameter"]["value"], {"kind": "SIGNED_POINTS", "value": 42}
        )
        self.assertTrue(
            all(
                reflection[k] == {"state": "UNKNOWN"}
                for k in ("target_domain", "evaluation_phase", "priority")
            )
        )

    def test_source_opposition_coordinates_raw_hash_and_corroboration(self):
        for mutate in [
            lambda e: e["own_source_boxes"].clear(),
            lambda e: e["own_source_boxes"][0]["reference"].update(revision_id=1),
            lambda e: e["own_source_boxes"][0].update(raw_sha256="0" * 64),
            lambda e: e["own_source_boxes"][0]["parameter_values"].update(
                itemid=["36656,36657"]
            ),
            lambda e: e["all_atom_own_xml_corroboration"][1][
                "all_vector_atoms_checked"
            ][0].update(typed_source_value=43),
        ]:
            e = copy.deepcopy(self.entries[36656])
            mutate(e)
            with self.assertRaises(ValueError):
                c.source_vector(
                    e, self.indexed, self.pages, "2026-09-27T23:59:59Z", self.order
                )
        e = copy.deepcopy(self.entries[36656])
        indexed = copy.deepcopy(self.indexed)
        indexed[36656].add(
            self.entries[36657]["own_source_boxes"][0]["reference"]["page_id"]
        )
        # A malformed own field mentioning this ID adds opposition even without a usable primary family.
        pages = copy.deepcopy(self.pages)
        pid = self.entries[36657]["own_source_boxes"][0]["reference"]["page_id"]
        pages[pid]["own_objects"][0]["raw_itemid_values"] = ["36656 invalid"]
        with self.assertRaises(ValueError):
            c.source_vector(e, indexed, pages, "2026-09-27T23:59:59Z", self.order)

    def test_current_target_headers_name_world_portability_and_vector_states(self):
        for mutate in [
            lambda a: a[1]["identity"].update(revision="wrong"),
            lambda a: a[1].update(materializable=True),
            lambda a: a[2].update(identity_namespace="wrong"),
            lambda a: a[3]["flags"].update({"flags.take": False}),
            lambda a: a[3]["flags"].update({"flags.unmove": True}),
            lambda a: a[5].add(a[0]["item_key"]),
            lambda a: a[1]["semantics"].update(presentation={"state": "UNKNOWN"}),
            lambda a: a[1]["semantics"].update(skill_modifiers={"state": "CONFLICT"}),
            lambda a: a[1]["semantics"].update(
                skill_modifiers={
                    "state": "KNOWN",
                    "value": {"modifiers": {"state": "KNOWN", "value": []}},
                }
            ),
        ]:
            args = self.args()
            mutate(args)
            with self.assertRaises(ValueError):
                c.qualify(*args)
        args = self.args()
        before = copy.deepcopy(args[1])
        expected = c.qualify(*args)
        args[1]["semantics"]["temporal"] = {"state": "CONFLICT"}
        self.assertEqual(
            c.qualify(*args), expected
        )  # unrelated source enrichment is not a full-native pin
        args[1]["semantics"]["skill_modifiers"] = {
            "state": "KNOWN",
            "value": {"modifiers": {"state": "KNOWN", "value": args[-1]}},
        }
        self.assertEqual(c.qualify(*args), expected)
        self.assertEqual(before, self.defs[args[0]["item_key"]])


if __name__ == "__main__":
    unittest.main()
