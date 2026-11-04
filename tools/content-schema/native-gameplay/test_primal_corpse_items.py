"""The Primal Menace uses its own pinned corpse capacities and decay chain."""

import copy
import json
from pathlib import Path
import tempfile
import unittest

import primal_corpse_items as primal

ROOT = Path(__file__).resolve().parents[3]
SHARD = ROOT / "content/items/definitions/items-24000-24499.json"


class PrimalCorpseItemTests(unittest.TestCase):
    def definitions(self):
        records = [row["definition"] for row in json.loads(SHARD.read_bytes())["records"]]
        template = next(
            row["definition"]
            for row in json.loads((ROOT / "content/items/definitions/items-29500-29999.json").read_bytes())["records"]
            if row["definition"]["identity"]["key"] == "oteryn:item.tibia.i5964"
        )
        return records + [template]

    def test_primal_corpse_has_complete_source_decay_chain_in_authored_data(self):
        records = {
            row["definition"]["identity"]["key"]: row["definition"]
            for row in json.loads(SHARD.read_bytes())["records"]
        }
        # Crystal201189f data/items/items.xml:73328-73347, and
        # appearances.dat object flags.container for these exact four IDs.
        for item, capacity, duration, target in (
            (39530, 30, 10_000, 39529),
            (39529, 30, 300_000, 39528),
            (39528, 15, 300_000, 39527),
            (39527, None, 60_000, None),
        ):
            with self.subTest(item=item):
                record = records[f"oteryn:item.tibia.i{item}"]
                self.assertTrue(record["materializable"])
                self.assertEqual(record["stack_class"], "NonStackable")
                semantics = record["semantics"]
                self.assertEqual(semantics["physical"], {"state": "KNOWN", "value": {
                    "weight": {"state": "KNOWN", "value": 0},
                    "movable": {"state": "KNOWN", "value": item != 39530},
                    "pickupable": {"state": "KNOWN", "value": False},
                }})
                self.assertEqual(semantics["stack"], {"state": "KNOWN", "value": {
                    "stackable": {"state": "KNOWN", "value": False},
                    "stack_max": {"state": "NOT_APPLICABLE"},
                }})
                self.assertEqual(
                    {key for key, value in semantics.items() if value["state"] != "UNKNOWN"},
                    {"physical", "stack", "container", "temporal"},
                    "No unrelated corpse, equipment or ability semantics may be copied",
                )
                self.assertEqual(
                    semantics["container"],
                    {"state": "KNOWN", "value": {"capacity": {"state": "KNOWN", "value": capacity}}}
                    if capacity is not None else {"state": "NOT_APPLICABLE"},
                )
                temporal = semantics["temporal"]["value"]
                self.assertEqual(temporal["stop_duration"], {"state": "KNOWN", "value": False})
                self.assertEqual(temporal["consumption_mode"], {
                    "state": "KNOWN", "value": "DURABLE_ABSOLUTE_DEADLINE"})
                self.assertEqual(temporal["duration"], {"state": "KNOWN", "value": duration})
                self.assertEqual(
                    temporal["decay_target"],
                    {"state": "KNOWN", "value": {
                        "key": f"oteryn:item.tibia.i{target}", "revision": "definition-r1"}}
                    if target is not None else {"state": "NOT_APPLICABLE"},
                )

    def test_overlay_preserves_unrelated_items_and_world_object_ownership(self):
        definitions = self.definitions()
        before = copy.deepcopy(definitions)
        object_path = ROOT / primal.OBJECT_SHARD
        world_before = object_path.read_bytes()
        primal.apply_admissions(definitions)
        owned = {f"oteryn:item.tibia.i{item}" for item in primal.OWNED}
        self.assertEqual(
            [record for record in definitions if record["identity"]["key"] not in owned],
            [record for record in before if record["identity"]["key"] not in owned],
        )
        self.assertEqual(object_path.read_bytes(), world_before)

    def test_mismatched_world_binding_refuses_before_mutating_any_item(self):
        definitions = self.definitions()
        before = copy.deepcopy(definitions)
        objects = json.loads((ROOT / primal.OBJECT_SHARD).read_bytes())
        terminal = next(row for row in objects["records"]
                        if row["identity"]["key"] == "oteryn:world-object.tibia.i39527")
        terminal["provenance"]["item_pointer"]["revision"] = "unqualified"
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / primal.OBJECT_SHARD
            target.parent.mkdir(parents=True)
            target.write_text(json.dumps(objects))
            with self.assertRaisesRegex(ValueError, "binding differs"):
                primal.apply_admissions(definitions, root)
        self.assertEqual(definitions, before)

    def test_committed_shard_is_reproducible(self):
        outputs = primal.outputs()
        self.assertEqual(set(outputs), {SHARD})
        self.assertEqual(outputs[SHARD], SHARD.read_bytes())


if __name__ == "__main__":
    unittest.main()
