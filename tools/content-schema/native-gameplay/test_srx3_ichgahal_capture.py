"""Ichgahal returns through its actual pinned OTBM teleport arrival."""

import json
import unittest
from pathlib import Path

import build_activity_source_world as source

ROOT = Path(__file__).resolve().parents[3]
RAW = [33004, 32343, 15]
FINAL = [34065, 31717, 15]


class IchgahalCaptureTests(unittest.TestCase):
    def setUp(self):
        self.capture = json.loads(
            (ROOT / "content/encounters/rotten-blood-ichgahal/source-room.json").read_bytes()
        )

    def tile(self, position):
        rows = [row for row in self.capture["tiles"] if row["source_position"] == position]
        self.assertEqual(len(rows), 1, f"one genuine captured tile at {position}")
        return rows[0]

    def test_raw_teleport_remains_pending_with_its_exact_destination(self):
        raw = self.tile(RAW)
        self.assertEqual(raw["semantics"]["unmaterialized_dynamic_items"], [34111])
        self.assertEqual(
            raw["items"],
            [
                {"attributes": {}, "depth": 0, "server_item_id": 42852},
                {"attributes": {"dest": FINAL}, "depth": 0, "server_item_id": 34111},
            ],
        )

    def test_arrival_retains_the_pinned_immutable_stack_and_known_ground(self):
        target = self.tile(FINAL)
        self.assertEqual(
            target["items"],
            [
                {"attributes": {}, "depth": 0, "server_item_id": 36344},
                {"attributes": {}, "depth": 0, "server_item_id": 292},
            ],
        )
        self.assertEqual(target["semantics"]["unmaterialized_dynamic_items"], [])
        self.assertEqual(target["semantics"]["ground"], 36344)
        self.assertEqual(target["semantics"]["ground_speed"], 150)
        self.assertFalse(any(target["semantics"]["flags"].values()))
        self.assertEqual(self.capture["source_revision"], source.PIN)
        self.assertEqual(
            {row["path"]: row["git_blob"] for row in self.capture["source_closure"]},
            {path: expected for path, (_, expected) in source.FILES.items()},
        )

    def test_generator_adds_only_the_actual_arrival_region(self):
        self.assertEqual(
            source.ROOMS["rotten-blood-ichgahal"]["regions"],
            [
                [32998, 32326, 33018, 32343, 15, 15],
                [32973, 32332, 32980, 32334, 15, 15],
                [34065, 31717, 34065, 31717, 15, 15],
            ],
        )
        self.assertEqual(
            self.capture["source_regions"],
            source.ROOMS["rotten-blood-ichgahal"]["regions"],
        )


if __name__ == "__main__":
    unittest.main()
