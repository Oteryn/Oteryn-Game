"""Source-specific M25 corpse policies cannot borrow another donor's capacity."""
from pathlib import Path
import copy
import json
import tempfile
import unittest

import m25_corpse_items as producer


class M25CorpseSourceTests(unittest.TestCase):
    def test_selected_source_top_order_is_frozen_before_registration(self):
        packet, _ = producer.checked_packet()
        for row in packet["records"]:
            with self.subTest(item=row["production_definition"]["production_key"]):
                self.assertIs(row["attributes"].get("always_on_top"), False)
                self.assertEqual(row["attributes"].get("always_on_top_order"), 0)

    def test_selected_donor_exact_chains_and_custody(self):
        packet, candidate = producer.checked_packet()
        records = {row["production_definition"]["production_key"]: row
                   for row in packet["records"]}
        self.assertEqual(set(records), {f"oteryn:item.tibia.i{i}"
                                      for i in (42069, 42072, 42071, 42070, 44687, 44686, 44685, 44684)})
        for item, capacity, duration, target in (
            (42069, 30, 10000, 42072), (42072, 30, 300000, 42071),
            (42071, 30, 300000, 42070), (42070, None, 60000, None),
            (44687, 20, 10000, 44686), (44686, 20, 300000, 44685),
            (44685, 10, 300000, 44684), (44684, None, 60000, None),
        ):
            with self.subTest(item=item):
                record = records[f"oteryn:item.tibia.i{item}"]
                self.assertEqual(record["admission"], {
                    "materializable": True, "stack_class": "NonStackable",
                    "legal_destinations": ["Ground"],
                })
                semantics = record["semantics"]
                self.assertEqual(semantics["physical"], {"state": "KNOWN", "value": {
                    "weight": {"state": "KNOWN", "value": 0},
                    "movable": {"state": "KNOWN", "value": item not in (42069, 44687)},
                    "pickupable": {"state": "KNOWN", "value": False},
                }})
                self.assertEqual(semantics["stack"], {"state": "KNOWN", "value": {
                    "stackable": {"state": "KNOWN", "value": False},
                    "stack_max": {"state": "NOT_APPLICABLE"},
                }})
                self.assertEqual(semantics["container"],
                    {"state": "KNOWN", "value": {"capacity": {"state": "KNOWN", "value": capacity}}}
                    if capacity is not None else {"state": "NOT_APPLICABLE"})
                if duration is None:
                    self.assertEqual(semantics["temporal"], {"state": "NOT_APPLICABLE"})
                else:
                    temporal = semantics["temporal"]["value"]
                    self.assertEqual(temporal["duration"], {"state": "KNOWN", "value": duration})
                    self.assertEqual(temporal["stop_duration"], {"state": "KNOWN", "value": False})
                    self.assertEqual(temporal["decay_target"],
                        {"state": "KNOWN", "value": {"key": f"oteryn:item.tibia.i{target}", "revision": "definition-r1"}}
                        if target is not None else {"state": "NOT_APPLICABLE"})

    def test_canary_existing_references_have_exact_independent_qualification(self):
        packet, candidate = producer.checked_packet()
        self.assertEqual([int(b["external_id"]) for b in candidate["bindings"]],
                         [44684, 44685, 44686, 44687])
        for row in packet["records"]:
            item = int(row["production_binding"]["external_id"])
            self.assertEqual(row["production_binding_qualification"],
                             candidate if item in producer.ORDINALS else None)
            self.assertEqual(row["production_binding"]["source_key"],
                             "oteryn:source.canary" if item in producer.ORDINALS
                             else "oteryn:source.crystalserver")

    def test_selected_donor_capacity_and_identity_tampering_are_rejected(self):
        packet, candidate = producer.checked_packet()
        changed = copy.deepcopy(packet)
        row = next(r for r in changed["records"] if r["production_binding"]["external_id"] == "44687")
        row["semantics"]["container"]["value"]["capacity"]["value"] = 8
        with self.assertRaisesRegex(ValueError, "M25_SOURCE_CAPACITY"):
            producer.validate(changed, candidate)
        changed = copy.deepcopy(candidate)
        changed["bindings"][0]["target"]["key"] = "oteryn:item.tibia.i44687"
        with self.assertRaisesRegex(ValueError, "M25_IDENTITY_AUTHORITY"):
            producer.validate(packet, changed)

    def test_missing_decay_stage_cannot_be_replaced_by_a_root_only_profile(self):
        packet, candidate = producer.checked_packet()
        packet["records"].pop()
        with self.assertRaisesRegex(ValueError, "M25_EXACT_RECORD_SET"):
            producer.validate(packet, candidate)

    def test_capture_byte_tampering_is_rejected_before_admission(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for path in (producer.PATH, producer.PACKET, producer.PIN_PATH):
                target = root / path
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes((producer.ROOT / path).read_bytes())
            raw = json.loads((root / producer.PATH).read_bytes())
            raw["records"][0]["attributes"]["blocks_projectile"] = True
            (root / producer.PATH).write_bytes(producer.encode(raw))
            with self.assertRaisesRegex(ValueError, "M25_CAPTURE_DRIFT"):
                producer.checked_packet(root)


if __name__ == "__main__":
    unittest.main()
