"""Ground projection must not replace or relax existing Item custody."""

import copy
import hashlib
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import bound_ground
import summer_ground
import world_objects


class BoundGroundTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.paths = [bound_ground.ROOT / "imports/crystalserver/bindings/items.json",
                     *sorted((bound_ground.ROOT / "content/items/definitions").glob("items-*.json"))]
        cls.before = {p: hashlib.sha256(p.read_bytes()).hexdigest() for p in cls.paths}
        cls.row = bound_ground.record(identifier=32691, expected_speed=150)

    def test_exact_ground_and_existing_item_identity_are_both_retained(self):
        self.assertEqual(self.row["identity"]["key"], "oteryn:terrain.tibia.i32691")
        self.assertEqual(self.row["provenance"]["item_pointer"],
                         {"family": "Item", "key": "oteryn:item.tibia.i32691", "revision": "definition-r1"})
        self.assertEqual(self.row["ground_speed"], {"state": "KNOWN", "value": 150})
        self.assertEqual(self.row["walkable"], {"state": "KNOWN", "value": True})
        self.assertEqual(world_objects.validate_record(self.row), [])
        self.assertEqual({p: hashlib.sha256(p.read_bytes()).hexdigest() for p in self.paths}, self.before)

    def test_changed_official_facts_or_item_pointer_are_rejected(self):
        for path, value in [("ground_speed", {"state": "KNOWN", "value": 100}),
                            ("walkable", {"state": "KNOWN", "value": False})]:
            changed = copy.deepcopy(self.row)
            changed[path] = value
            self.assertTrue(world_objects.validate_record(changed), path)
        changed = copy.deepcopy(self.row)
        changed["provenance"]["item_pointer"]["key"] = "oteryn:item.tibia.i32692"
        self.assertTrue(world_objects.validate_record(changed))

    def test_missing_or_retargeted_binding_is_rejected(self):
        original = Path.read_bytes
        for replacement in [None, {"family": "Item", "key": "oteryn:item.tibia.i32692", "revision": "definition-r1"}]:
            def altered(path):
                raw = original(path)
                if path == self.paths[0]:
                    document = json.loads(raw)
                    if replacement is None:
                        document["bindings"] = [r for r in document["bindings"] if r["external_id"] != "32691"]
                    else:
                        next(r for r in document["bindings"] if r["external_id"] == "32691")["target"] = replacement
                    return json.dumps(document).encode()
                return raw
            with patch.object(Path, "read_bytes", altered):
                with self.assertRaisesRegex(ValueError, "requires one exact Item binding"):
                    bound_ground.record(identifier=32691, expected_speed=150)

    def test_source_speed_drift_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "accepted appearance"):
            bound_ground.record(identifier=32691, expected_speed=100)

    def test_appearance_only_admission_still_rejects_item_bound_ground(self):
        with self.assertRaisesRegex(ValueError, "unexpectedly has an Item binding"):
            summer_ground.outputs(identifier=32691, expected_speed=150)


if __name__ == "__main__":
    unittest.main()
