"""Source scene decoration never manufactures inventory Item identity."""

import copy
import json
import unittest
from pathlib import Path

import activity_static
import world_objects

ROOT = Path(__file__).resolve().parents[3]


class ActivityStaticTests(unittest.TestCase):
    def test_m25_exact_source_static_cohort_is_registered_without_item_aliases(self):
        # Actual pinned OTBM room+waiting+exit stacks; each source.policies row
        # is immutable, nonpickupable and has no dynamic Item owner.
        expected = frozenset(
            [
                34628,
                40242,
                40251,
                40252,
                40282,
                40309,
                40310,
                40311,
                40312,
                40313,
                40314,
                40315,
                40316,
                40317,
                40318,
                40319,
                40320,
                40321,
                40322,
                40323,
                40329,
                40332,
                40333,
                40334,
                40337,
                40338,
                40339,
                40340,
                40341,
                40342,
                40343,
                40344,
                40345,
                40346,
                40347,
                40348,
                40351,
                40352,
                40353,
                40360,
                40361,
                40387,
                40388,
                40391,
                40393,
                40397,
                40400,
                40405,
                40406,
                40414,
                40437,
                40439,
                40449,
                40454,
                40456,
                40471,
                40472,
                40473,
                40474,
                40477,
                40478,
                40479,
                40480,
                40492,
                44481,
                44482,
                44483,
                44485,
                44489,
                44491,
                44493,
                44494,
                44498,
                44499,
                44500,
                44502,
            ]
        )
        self.assertTrue(expected <= activity_static.IDENTIFIERS)
        self.assertEqual(activity_static.M25_IDENTIFIERS, expected)
        self.assertEqual(len(expected), 76)

    def test_all_capture_static_ids_have_non_item_publication(self):
        keys = {
            row[0]
            for row in json.loads(
                (
                    ROOT / "tools/content-schema/native-gameplay/item-keys.json"
                ).read_bytes()
            )["records"]
        }
        missing = set()
        for path in (ROOT / "content/encounters").glob("*/source-room.json"):
            for binding in json.loads(path.read_bytes())["source_identity_bindings"]:
                identifier = int(binding["external_id"])
                if f"oteryn:item.tibia.i{identifier}" not in keys:
                    missing.add(identifier)
        # Restore-only source ground need not be present in the initial tile stack.
        anomaly = json.loads(
            (ROOT / "content/encounters/callback-anomaly/source-room.json").read_bytes()
        )
        self.assertTrue(
            any(p["server_item_id"] == 23049 for p in anomaly["item_policies"])
        )
        missing.add(23049)
        registry = json.loads(
            (ROOT / "content/world/source-static-definitions.json").read_bytes()
        )
        missing.add(33276)
        self.assertEqual(
            {row["source_item_id"] for row in registry["records"]}, missing
        )
        self.assertEqual(len(missing), 238)
        for row in registry["records"]:
            self.assertIn(row["identity"]["family"], ["Terrain", "WorldObject"])
            self.assertFalse(row["policy"]["pickupable"])
            self.assertFalse(row["policy"]["movable"])
            self.assertIsNone(row["policy"]["dynamic_kind"])

    def test_ebb_source_decoration_keeps_one_exact_canonical_object_body(self):
        registry = json.loads(
            (ROOT / "content/world/source-static-definitions.json").read_bytes()
        )
        row = next(
            (r for r in registry["records"] if r["source_item_id"] == 33276), None
        )
        self.assertIsNotNone(
            row, "actual three Ebb maps require pinned static33276 facts"
        )
        self.assertEqual(row["identity"]["family"], "WorldObject")
        self.assertFalse(row["policy"]["block_solid"])
        self.assertFalse(row["policy"]["movable"])
        self.assertFalse(row["policy"]["pickupable"])
        self.assertNotIn("item_pointer", row["catalogue_record"]["provenance"])
        actual = [
            r
            for p in (ROOT / "content/world/objects").glob("objects-*.json")
            for r in json.loads(p.read_bytes()).get("records", [])
            if r["identity"]["key"] == row["identity"]["key"]
        ]
        self.assertEqual(
            actual,
            [row["catalogue_record"]],
            "publish exactly one canonical source body, never a duplicate",
        )

    def test_anomaly_restore_has_one_exact_canonical_terrain_body(self):
        registry = json.loads(
            (ROOT / "content/world/source-static-definitions.json").read_bytes()
        )
        expected = next(
            r["catalogue_record"]
            for r in registry["records"]
            if r["source_item_id"] == 23049
        )
        actual = [
            r
            for p in (ROOT / "content/world/terrain").glob("terrain-*.json")
            for r in json.loads(p.read_bytes()).get("records", [])
            if r["identity"]["key"] == expected["identity"]["key"]
        ]
        self.assertEqual(
            actual,
            [expected],
            "source registry must have one canonical exact body, not an appended duplicate",
        )

    def test_anomaly_restore_ground_is_sealed_source_surface(self):
        registry = json.loads(
            (ROOT / "content/world/source-static-definitions.json").read_bytes()
        )
        record = next(
            (r for r in registry["records"] if r["source_item_id"] == 23049), None
        )
        self.assertIsNotNone(
            record,
            "actual Anomaly restore target must have pinned source terrain facts",
        )
        self.assertEqual(record["identity"]["family"], "Terrain")
        self.assertNotIn("item_pointer", record["catalogue_record"]["provenance"])
        self.assertEqual(record["policy"]["ground_speed"], 150)
        self.assertFalse(record["policy"]["block_projectile"])

    def test_vemiath_teleport_destination_is_actual_static_source_tile(self):
        capture = json.loads(
            (
                ROOT / "content/encounters/rotten-blood-vemiath/source-room.json"
            ).read_bytes()
        )
        target = next(
            (
                tile
                for tile in capture["tiles"]
                if tile["source_position"] == [34001, 31820, 15]
            ),
            None,
        )
        self.assertIsNotNone(
            target, "source34111 Teleport arrival must be a genuine captured OTBM tile"
        )
        self.assertEqual(
            [item["server_item_id"] for item in target["items"]], [36344, 42671]
        )
        registry = json.loads(
            (ROOT / "content/world/source-static-definitions.json").read_bytes()
        )
        record = next(
            row for row in registry["records"] if row["source_item_id"] == 42671
        )
        self.assertEqual(record["identity"]["family"], "WorldObject")
        self.assertNotIn("item_pointer", record["catalogue_record"]["provenance"])
        self.assertFalse(record["policy"]["movable"])
        self.assertFalse(record["policy"]["pickupable"])
        self.assertIsNone(record["policy"]["dynamic_kind"])

    def test_published_source_collision_and_no_item_pointer_are_protected(self):
        rows = json.loads(
            activity_static.outputs()[activity_static.SHARDS["WorldObject"]]
        )["records"]
        actual = next(
            row for row in rows if row["provenance"]["source_item_id"] == 39095
        )
        self.assertEqual(world_objects.validate_record(actual), [])
        changed = copy.deepcopy(actual)
        changed["collision"]["blocks_movement"]["value"] = not changed["collision"][
            "blocks_movement"
        ]["value"]
        self.assertTrue(world_objects.validate_record(changed))
        changed = copy.deepcopy(actual)
        changed["provenance"]["item_pointer"] = {
            "family": "Item",
            "key": "oteryn:item.tibia.i39095",
            "revision": "definition-r1",
        }
        self.assertTrue(
            world_objects.validate_record(changed),
            "appearance geometry cannot mint an Item pointer",
        )


if __name__ == "__main__":
    unittest.main()
