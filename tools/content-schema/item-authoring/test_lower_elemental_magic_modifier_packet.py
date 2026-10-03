"""Whole raw presence, fixed identity, source ordering and native-state guards."""

import copy
import json
import unittest

import lower_elemental_magic_modifier_packet as compiler
from engine_items import decode_appearance_object, protobuf_fields


class ModifierTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.proof = json.loads((compiler.ROOT / compiler.PROOF).read_text())
        cls.entries = {e["item_id"]: e for e in cls.proof["records"]}
        cls.order = compiler.native_order()
        cls.objects = {}
        for tag, raw in protobuf_fields(
            (compiler.ROOT / compiler.base.CLIENT).read_bytes()
        ):
            if tag == 1:
                obj = decode_appearance_object(raw)
                cls.objects[obj["id"]] = obj | {"object_sha256": compiler.base.sha(raw)}
        cls.defs = {
            r["definition"]["identity"]["key"]: r["definition"]
            for path in json.loads(
                (compiler.ROOT / "content/items/index.json").read_text()
            )["shards"]
            for r in json.loads((compiler.ROOT / path).read_text())["records"]
        }

    def args(self, iid=43886):
        e = copy.deepcopy(self.entries[iid])
        return [
            e,
            copy.deepcopy(self.defs[e["item_key"]]),
            copy.deepcopy(e["binding"]),
            copy.deepcopy(self.objects[iid]),
            copy.deepcopy(e["retained_observations"]),
            set(),
            self.order,
            self.proof["source_cutoff"],
        ]

    def test_source_scope_and_actual_native_order(self):
        packet = compiler.build()
        self.assertEqual(
            packet["counts"], {"fields": 26, "items": 26, "atoms": 63, "holds": 23}
        )
        self.assertEqual(
            {r["target"]["key"] for r in packet["promotions"]},
            {self.entries[i]["item_key"] for i in compiler.IDS},
        )
        kinds = [
            e["kind"] for e in compiler.qualify(*self.args())["typed_value"]["value"]
        ]
        self.assertLess(
            kinds.index("CRITICAL_HIT_DAMAGE"), kinds.index("EARTH_MAGIC_LEVEL_POINTS")
        )
        self.assertTrue(
            all(
                compiler.qualify(*self.args(i)) is None
                for i in self.entries
                if i not in compiler.IDS
            )
        )

    def test_present_empty_unsupported_duplicates_and_point_units(self):
        for fields in [
            {"attrib": ["holy magic level +1"], "crithit_ch": [""]},
            {"attrib": ["holy magic level +1"], "mantra": [""]},
            {"attrib": ["holy magic level +1, holy magic level +2"]},
            {"attrib": ["holy magic level +1%"]},
            {"attrib": ["holy magic level 2147483648"]},
            {"attrib": ["holy magic level +1"], "hpleech_am": ["1%", "2%"]},
        ]:
            self.assertIsNone(compiler.whole_vector(fields, self.order))

    def test_explicit_vector_parser_preserves_guards_and_rejection(self):
        self.assertEqual(
            compiler.qualify(*self.args()),
            compiler.qualify(*self.args(), vector_parser=compiler.whole_vector),
        )
        calls = []

        def reject(fields, order):
            calls.append((fields, order))

        self.assertIsNone(compiler.qualify(*self.args(), vector_parser=reject))
        self.assertEqual(len(calls), 1)
        calls.clear()
        args = self.args()
        args[3]["object_sha256"] = "0" * 64
        self.assertIsNone(compiler.qualify(*args, vector_parser=reject))
        self.assertEqual(calls, [])

    def test_identity_variant_raw_source_coordinates_domain_holds(self):
        for index, key, value in [
            (0, "item_id", 52789),
            (2, "disposition", "UNRESOLVED"),
            (2, "identity_namespace", "wrong"),
            (3, "name", "wrong"),
            (3, "object_sha256", "0" * 64),
        ]:
            args = self.args()
            args[index][key] = value
            self.assertIsNone(compiler.qualify(*args))
        for flag in ["flags.take", "flags.unmove", "flags.clip"]:
            args = self.args()
            args[3]["flags"][flag] = flag != "flags.take"
            self.assertIsNone(compiler.qualify(*args))
        args = self.args()
        args[5].add(args[0]["item_key"])
        self.assertIsNone(compiler.qualify(*args))
        for coordinate in [
            "page_id",
            "revision_id",
            "revision_timestamp",
            "content_sha256",
            "wiki_title",
        ]:
            args = self.args()
            args[4][0][coordinate] = "drift"
            self.assertIsNone(compiler.qualify(*args))
        args = self.args()
        args[0]["own_boxes"][0]["parameter_values"]["itemid"] = ["43886,43885"]
        self.assertIsNone(compiler.qualify(*args))

    def test_native_blocked_conflicting_context_and_unknown_names_preserved(self):
        for path in ["skill_modifiers", "presentation"]:
            for state in ["CONFLICT", "NOT_APPLICABLE"]:
                args = self.args()
                args[1]["semantics"][path] = {"state": state}
                self.assertIsNone(compiler.qualify(*args))
        args = self.args()
        args[1]["semantics"]["skill_modifiers"] = {
            "state": "KNOWN",
            "value": {"modifiers": {"state": "KNOWN", "value": []}},
        }
        self.assertIsNone(compiler.qualify(*args))
        for iid in [53219, 53220, 53221, 53222, 53230]:
            args = self.args(iid)
            before = copy.deepcopy(args[1])
            row = compiler.qualify(*args)
            self.assertIsNotNone(row)
            self.assertEqual(args[1], before)
            self.assertEqual(
                compiler.leaf(args[1], "presentation.name"), {"state": "UNKNOWN"}
            )
            self.assertTrue(
                all(
                    e[k] == {"state": "UNKNOWN"}
                    for e in row["typed_value"]["value"]
                    for k in ["target_domain", "evaluation_phase", "priority"]
                )
            )


if __name__ == "__main__":
    unittest.main()
