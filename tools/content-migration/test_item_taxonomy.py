"""Source agreement, identity boundaries and post-Wave-1 relation regressions."""

import unittest

from item_taxonomy import build_taxonomy, legacy_profile
from world_project_v2_to_tree import capability_relations


class TaxonomyTests(unittest.TestCase):
    def rows(self, categories, bound=True, routed=False, legacy=None, **sources):
        target = {
            "family": "Item",
            "key": "oteryn:item.tibia.i34086",
            "revision": "definition-r1",
        }
        key = tuple(target.values())
        observations = [
            {
                "fields": {"primarytype": category},
                "page_id": i,
                "revision_id": i + 1,
                "content_sha256": "a" * 64,
                "url": "https://tibia.fandom.com/wiki/Soulcrusher",
            }
            for i, category in enumerate(categories)
        ]
        snapshot = {
            "snapshot_sha256": "b" * 64,
            "records": {"34086": {"item_id": 34086, "observations": observations}},
        }
        authoring = (
            {key: {"item": target, "taxonomy": {"primary": legacy}}} if legacy else {}
        )
        return build_taxonomy(
            {key: {"identity": target}},
            authoring,
            {"Clavas": "weapon_melee"},
            snapshot,
            {target["key"]} if bound else set(),
            {target["key"]} if routed else set(),
            **sources,
        )

    def test_source_agreement_and_provenance(self):
        rows = self.rows(["Club Weapons", "Sword Weapons"])
        self.assertEqual(rows[0]["family_profile"], "weapon_melee")
        self.assertEqual(len(rows[0]["source_evidence"]["observations"]), 2)

    def test_conflicting_unknown_and_appearance_only_are_held(self):
        for categories, bound, routed in [
            (["Club Weapons", "Food"], True, False),
            (["Club Weapons", "Not a category"], True, False),
            (["Club Weapons"], False, False),
            (["Plants"], True, True),
        ]:
            self.assertEqual(self.rows(categories, bound, routed), [])

    def test_existing_br_taxonomy_is_not_overwritten(self):
        row = self.rows(["Food"], legacy="Clavas")[0]
        self.assertEqual(row["family_profile"], "weapon_melee")
        self.assertNotIn("source_evidence", row)
        self.assertEqual(legacy_profile("Armas de Arremesso", {}), "weapon_distance")

    def test_client_agreement_and_identity_holds(self):
        client = {
            34086: {
                "name": "monk robe",
                "flags": {"flags.take": True, "market.category": 1, "clothes.slot": 4},
            }
        }
        row = self.rows([], client=client)[0]
        self.assertEqual(row["family_profile"], "equipment_armor")
        self.assertEqual(row["source_evidence"]["appearance_id"], 34086)
        for kwargs in ({"bound": False}, {"routed": True}):
            self.assertEqual(self.rows([], client=client, **kwargs), [])
        self.assertEqual(self.rows(["Unknown category"], client=client), [])
        client[34086]["flags"]["market.category"] = 7
        self.assertEqual(self.rows([], client=client), [])
        client[34086]["flags"]["market.category"] = 1
        client[34086]["flags"]["clothes.slot"] = 9
        self.assertEqual(self.rows([], client=client), [])
        client[34086]["flags"].pop("clothes.slot")
        self.assertEqual(self.rows([], client=client), [])

    def test_admitted_fallback_preserves_evidence_and_primary_holds(self):
        fallback = {
            "oteryn:item.tibia.i34086": {
                "profile": "quest_item",
                "snapshot_sha256": "c" * 64,
                "evidence": {
                    "resolution": "disambiguation",
                    "candidates": [
                        {"field": "primarytype", "value": "Quest Items"},
                        {"field": "primarytype", "value": "Quest Objects"},
                    ],
                },
            }
        }
        row = self.rows([], fallback=fallback)[0]
        self.assertEqual(row["family_profile"], "quest_item")
        self.assertEqual(
            row["source_evidence"]["qualified_fallback"],
            fallback["oteryn:item.tibia.i34086"]["evidence"],
        )
        self.assertEqual(self.rows(["Unknown category"], fallback=fallback), [])
        self.assertEqual(self.rows(["Club Weapons", "Food"], fallback=fallback), [])

    def test_later_imbuement_slots_need_no_wave1_authoring(self):
        definition = {
            "semantics": {
                "imbuement": {
                    "state": "KNOWN",
                    "value": {"slot_count": {"state": "KNOWN", "value": 2}},
                }
            }
        }
        self.assertEqual(
            capability_relations(definition, None),
            [
                {
                    "relation": "CAPABILITY_GOVERNED_BY",
                    "ruleset": "rulesets/items/imbuements/",
                    "basis": "imbuement.slot_count>=1",
                }
            ],
        )
        definition["semantics"]["imbuement"]["value"]["slot_count"] = {
            "state": "UNKNOWN"
        }
        self.assertEqual(capability_relations(definition, None), [])


if __name__ == "__main__":
    unittest.main()
