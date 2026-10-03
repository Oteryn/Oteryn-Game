"""Source agreement, identity boundaries and post-Wave-1 relation regressions."""

import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from item_taxonomy import (
    BULK_FALLBACK_PATH,
    FALLBACK_PATH,
    build_taxonomy,
    legacy_profile,
    load_taxonomy_fallback,
)
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

    def test_extra_slot_uses_admitted_equipment_family_not_quest_origin(self):
        root = Path(__file__).resolve().parents[2]
        profiles = json.loads(
            (
                root / "tools/content-schema/item-authoring/profile-catalog.json"
            ).read_text()
        )
        entries = profiles["profiles"]
        profile = next(
            row for row in entries if row["profile_id"] == "equipment_offhand"
        )
        self.assertIn("Extra Slot", profile["navigation_families"])
        row = self.rows(["Extra Slot"])[0]
        self.assertEqual(row["family_profile"], profile["profile_id"])
        self.assertEqual(
            row["source_evidence"]["observations"][0]["primarytype"], "Extra Slot"
        )
        self.assertEqual(self.rows(["Extra Slot", "Quest Items"]), [])

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

    def test_bulk_fallback_uses_own_source_path_and_retains_old_path(self):
        key = "oteryn:item.tibia.i34086"
        entry = {
            "profile": "quest_item",
            "snapshot_sha256": "c" * 64,
            "evidence": {
                "resolution": "direct",
                "field": "primarytype",
                "value": "Quest Items",
            },
        }
        old = self.rows([], fallback={key: entry})[0]
        self.assertEqual(old["source_evidence"]["snapshot"], FALLBACK_PATH)
        entry["snapshot_path"] = BULK_FALLBACK_PATH
        new = self.rows([], fallback={key: entry})[0]
        self.assertEqual(new["source_evidence"]["snapshot"], BULK_FALLBACK_PATH)
        self.assertEqual(new["source_evidence"]["snapshot_sha256"], "c" * 64)
        self.assertEqual(self.rows([], fallback={key: entry}, routed=True), [])
        self.assertEqual(self.rows(["Unknown category"], fallback={key: entry}), [])

    def test_separate_snapshot_rejects_overlap_owner_binding_and_primary_replacement(
        self,
    ):
        key = "oteryn:item.tibia.i34086"
        record = {
            "registry_key": key,
            "matched_names": ["soulcrusher"],
            "resolution": "direct",
            "match_basis": "title",
            "field": "primarytype",
            "value": "Quest Items",
            "wiki_title": "Soulcrusher",
            "page_id": 1,
            "revision_id": 2,
            "revision_timestamp": "2026-09-27T00:00:00Z",
            "url": "https://tibia.fandom.com/wiki/Soulcrusher",
            "revision_sha1": "a" * 40,
            "content_sha256": "b" * 64,
            "captured_at": "2026-10-01T00:00:00Z",
        }

        def write_snapshot(root, path, records):
            digest = hashlib.sha256(
                json.dumps(
                    records, ensure_ascii=False, sort_keys=True, separators=(",", ":")
                ).encode()
            ).hexdigest()
            target = root / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(
                json.dumps(
                    {
                        "schema": "OTERYN_ITEM_FAMILY_FALLBACK_SNAPSHOT/v1",
                        "batch_id": "test",
                        "family": "Item",
                        "source": {},
                        "captured_at": "2026-10-01T00:00:00Z",
                        "records": records,
                        "snapshot_sha256": digest,
                    }
                )
            )

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_snapshot(root, FALLBACK_PATH, {})
            write_snapshot(root, BULK_FALLBACK_PATH, {key: record})
            identity = {34086: (key, "crystal_exact_binding")}
            snapshot = {"records": {}}
            entry = load_taxonomy_fallback(root, identity, {key}, set(), snapshot)[key]
            self.assertEqual(entry["snapshot_path"], BULK_FALLBACK_PATH)
            for bound, owners, observed, expected in (
                (set(), set(), snapshot, "EXACT_BINDING_REQUIRED"),
                ({key}, {key}, snapshot, "WORLD_OWNER_PRECEDENCE"),
                (
                    {key},
                    set(),
                    {
                        "records": {
                            "34086": {
                                "item_id": 34086,
                                "observations": [
                                    {"fields": {"primarytype": "Unknown"}}
                                ],
                            }
                        }
                    },
                    "RETAINED_PRIMARY_PRECEDENCE",
                ),
            ):
                with self.assertRaisesRegex(ValueError, expected):
                    load_taxonomy_fallback(root, identity, bound, owners, observed)
            write_snapshot(root, FALLBACK_PATH, {key: record})
            with self.assertRaisesRegex(ValueError, "RETAINED_FALLBACK_OVERLAP"):
                load_taxonomy_fallback(root, identity, {key}, set(), snapshot)

    def test_owner_decision_is_reached_without_engine_family_and_respects_owner(self):
        target = {
            "family": "Item",
            "key": "oteryn:item.tibia.i15151",
            "revision": "definition-r1",
        }
        key = tuple(target.values())
        definition = {
            "identity": target,
            "semantics": {
                "presentation": {
                    "state": "KNOWN",
                    "value": {"name": {"state": "KNOWN", "value": "toad stool"}},
                }
            },
        }
        snapshot = {"records": {}, "snapshot_sha256": "b" * 64}
        args = ({key: definition}, {}, {}, snapshot, {target["key"]})
        row = build_taxonomy(*args, set())[0]
        self.assertEqual(row["family_profile"], "decoration")
        self.assertEqual(row["source_evidence"]["scope"], "NAVIGATION_ONLY")
        self.assertEqual(build_taxonomy(*args, {target["key"]}), [])
        self.assertEqual(
            build_taxonomy({key: definition}, {}, {}, snapshot, set(), set()), []
        )
        definition["semantics"]["presentation"]["value"]["name"]["value"] = (
            "different item"
        )
        self.assertEqual(build_taxonomy(*args, set()), [])


if __name__ == "__main__":
    unittest.main()
