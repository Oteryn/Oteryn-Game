import copy
import json
import sys
import tempfile
import unittest
from pathlib import Path

import feroxa_static
import world_objects


class FeroxaStaticTest(unittest.TestCase):
    def test_source_decoration_qualifies_without_portable_item_alias(self):
        (record,) = feroxa_static.records()
        self.assertEqual(world_objects.validate_record(record), [])
        self.assertEqual(record["identity"]["key"], "oteryn:terrain.tibia.i13841")
        self.assertNotIn("item_pointer", record["provenance"])
        self.assertEqual(record["kind"]["value"], "border")
        for field in ("walkable", "blocks_projectile", "client_projection"):
            altered = copy.deepcopy(record)
            altered[field] = {"state": "UNKNOWN"}
            self.assertTrue(world_objects.validate_record(altered), field)

    def test_projection_preserves_every_palette_slot_and_other_identities(self):
        before = json.loads(
            (feroxa_static.ROOT / "content/world/placements/index.json").read_bytes()
        )
        after = json.loads(
            feroxa_static.outputs()["content/world/placements/index.json"]
        )
        self.assertEqual(len(before["palette"]), len(after["palette"]))
        for a, b in zip(before["palette"], after["palette"]):
            self.assertEqual(a["source_item_id"], b["source_item_id"])
            if a["source_item_id"] != feroxa_static.IDENTIFIER:
                self.assertEqual(a, b)
            else:
                self.assertFalse(b["provisional"])
                self.assertEqual(b["key"], feroxa_static.KEY)

    def test_historical_replay_requires_exact_new_world_witness(self):
        sys.path.insert(
            0, str(feroxa_static.ROOT / "tools/content-schema/item-authoring")
        )
        import historical_world_owner_context as historical

        self.assertEqual(
            historical.sha(historical.original_marker(feroxa_static.ROOT)),
            historical.BASE_SHA,
        )
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in ("imports", "content/assets", "crates"):
                destination = root / name
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.symlink_to(
                    (feroxa_static.ROOT / name).resolve(), target_is_directory=True
                )
            for name in (
                "content/world/placements/index.json",
                "content/world/terrain/index.json",
            ):
                destination = root / name
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.symlink_to((feroxa_static.ROOT / name).resolve())
            (record,) = feroxa_static.records()
            record["walkable"]["value"] = False
            (root / feroxa_static.SHARD).write_text(
                json.dumps({"family": "Terrain", "records": [record]})
            )
            with self.assertRaisesRegex(ValueError, "Feroxa static World witness"):
                historical.original_marker(root)


if __name__ == "__main__":
    unittest.main()
