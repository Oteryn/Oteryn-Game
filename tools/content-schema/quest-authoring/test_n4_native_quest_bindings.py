"""N4 Crystal startup selectors must survive world callback lowering."""

import json
import unittest
from pathlib import Path

import n4_yalahar

ROOT = Path(__file__).resolve().parents[3]


class NativeYalaharMechanisms(unittest.TestCase):
    def test_startup_uids_resolve_to_native_mechanisms(self):
        rows = json.loads(n4_yalahar.generate(ROOT))["callbacks"]
        mechanisms = {
            int(row["key"].rsplit("-", 1)[1]): row
            for row in rows
            if "/in_service_of_yalahar/mechanism-" in row["key"]
        }
        self.assertEqual(set(mechanisms), {1018, 1019, *range(3091, 3103)})
        for uid, row in mechanisms.items():
            with self.subTest(uid=uid):
                self.assertEqual(row["trigger"], "use")
                self.assertEqual(row["selector"]["ordinal"], 1)
                self.assertIsNone(row["selector"]["unique"])
                self.assertEqual(row["effects"]["teleport_effect"], 11)
                self.assertTrue(row["effects"]["teleport_departure_effect"])
                if uid in (1018, 1019):
                    self.assertEqual(row["storage"], [])
                else:
                    self.assertEqual(len(row["storage"]), 1)
                    self.assertTrue(row["storage"][0]["read_only"])
        selected = mechanisms[3092]
        self.assertEqual(
            selected["selector"]["position"], {"x": 32743, "y": 31161, "floor": -5}
        )
        self.assertEqual(selected["storage"][0]["minimum"], 21)
        self.assertEqual(selected["teleport"], {"x": 32744, "y": 31164, "floor": -5})
