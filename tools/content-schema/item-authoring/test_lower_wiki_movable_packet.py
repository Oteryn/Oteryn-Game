"""Explicit source literals, whole-own-ID opposition and portable native guards."""

import copy
import json
import unittest

import lower_wiki_movable_packet as lower


class MovableTests(unittest.TestCase):
    def setUp(self):
        self.identity = {"family": "Item", "key": "item", "revision": "definition-r1"}
        self.page = {
            "page_id": 1,
            "title": "Object",
            "revision_id": 2,
            "revision_timestamp": "2026-09-01T00:00:00Z",
            "content_sha256": "a" * 64,
        }
        self.box = {
            "balanced": True,
            "inside_comment": False,
            "positive_exact_infobox_object_match": True,
        }
        self.fields = {
            "itemid": ["7"],
            "actualname": ["object"],
            "name": ["Object (Variant)"],
            "immobile": ["no"],
        }
        self.args = {
            "iid": 7,
            "definition": {"identity": self.identity},
            "binding": {"external_id": "7", "target": self.identity},
            "obj": {"name": "object", "flags": {"flags.take": True}},
            "routed": set(),
            "observations": [self.page | {"wiki_title": "Object"}],
            "pages": [(self.page, self.box, self.fields)],
        }

    def check(self, **updates):
        return lower.reasons(**(self.args | updates))[0]

    def test_exact_actualname_priority_no_absence_inference_and_idempotence(self):
        self.assertFalse(self.check())
        for values in (
            {"immobile": []},
            {"immobile": ["no", "yes"]},
        ):
            self.assertTrue(
                self.check(pages=[(self.page, self.box, self.fields | values)])
            )
        fallback = self.fields | {"name": ["object"]}
        self.assertTrue(
            self.check(pages=[(self.page, self.box, fallback | {"actualname": [""]})])
        )
        fallback.pop("actualname")
        self.assertFalse(self.check(pages=[(self.page, self.box, fallback)]))
        absent = {k: v for k, v in self.fields.items() if k != "immobile"}
        self.assertTrue(self.check(pages=[(self.page, self.box, absent)]))
        known = {
            "state": "KNOWN",
            "value": {"movable": {"state": "KNOWN", "value": True}},
        }
        self.assertFalse(
            self.check(
                definition={"identity": self.identity, "semantics": {"physical": known}}
            )
        )

    def test_all_competing_own_pages_include_world_yes_and_shared_id(self):
        for values in ({"immobile": ["yes"]}, {"itemid": ["7, 8"]}):
            other = (self.page | {"page_id": 3}, self.box, self.fields | values)
            self.assertTrue(self.check(pages=self.args["pages"] + [other]))
        self.assertTrue(self.check(routed={"item"}))
        self.assertTrue(
            self.check(
                obj=self.args["obj"]
                | {"flags": {"flags.take": True, "flags.unmove": True}}
            )
        )

    def test_binding_coordinates_names_and_native_blocks_cannot_be_substituted(self):
        for update in (
            None,
            self.args["binding"] | {"external_id": "8"},
            self.args["binding"] | {"target": self.identity | {"revision": "other"}},
        ):
            self.assertTrue(self.check(binding=update))
        for name in (*lower.COORDS, "wiki_title"):
            self.assertTrue(
                self.check(
                    observations=[self.args["observations"][0] | {name: "other"}]
                )
            )
        for state in ("CONFLICT", "NOT_APPLICABLE"):
            self.assertTrue(
                self.check(
                    definition={
                        "identity": self.identity,
                        "semantics": {"physical": {"state": state}},
                    }
                )
            )
        for path in ("movable", "pickupable"):
            self.assertTrue(
                self.check(
                    definition={
                        "identity": self.identity,
                        "semantics": {
                            "physical": {
                                "state": "KNOWN",
                                "value": {path: {"state": "KNOWN", "value": False}},
                            }
                        },
                    }
                )
            )
        self.assertTrue(
            self.check(
                pages=[
                    (self.page, self.box, self.fields | {"actualname": ["different"]})
                ]
            )
        )

    def test_complete_projection_raw_digest_and_deterministic_packet(self):
        proof = json.loads((lower.ROOT / lower.PROOF).read_text())
        indexed, pages = lower.own_index(proof)
        self.assertEqual((len(indexed), len(pages)), (13906, 9980))
        bad = copy.deepcopy(proof)
        bad["complete_own_id_occurrence_projection"].pop()
        with self.assertRaises(ValueError):
            lower.own_index(bad)
        packet = lower.build()
        self.assertEqual(
            packet["counts"], {"promotions": 5692, "fields": 5692, "holds": 12}
        )
        held = {r["source_item_id"]: r["reasons"] for r in packet["holds"]}
        for iid in (2984, 47397):
            self.assertIn("COMPETING_IMMOBILE_VALUE", held[iid])


if __name__ == "__main__":
    unittest.main()
