"""Boundary regressions for the actual six-entry copied death capture."""
import json
import unittest
from copy import deepcopy

import build_srx3_rotten_death_events as capture


class RottenDeathCaptureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.frozen = json.loads(capture.OUTPUT.read_bytes())

    def test_actual_vectors_keep_unloaded_index_and_only_six_covered_sources(self):
        capture.qualify_capture(self.frozen)
        rows = self.frozen["records"]
        self.assertEqual([row["creature_key"] for row in rows],
                         ["oteryn:creature." + key for key in capture.KEYS])
        self.assertEqual([(row["creature_key"], [(event["registration_index"], event["name"])
                                               for event in row["on_death"]]) for row in rows], [
            ("oteryn:creature.bakragore", [(1, "BakragoreMechanicsDeath")]),
            ("oteryn:creature.chagorz", [(1, "ChagorzMechanicsDeath")]),
            ("oteryn:creature.ichgahal", []), ("oteryn:creature.murcion", []),
            ("oteryn:creature.mushroom", []),
            ("oteryn:creature.vemiath", [(1, "VemiathMechanicsDeath")]),
        ])

    def test_refresh_requalifies_real_native_input_without_changing_source_capture(self):
        self.assertEqual(capture.refresh_native_pin(), self.frozen)

    def test_a_loaded_handler_cannot_be_dropped_or_moved_to_the_unloaded_slot(self):
        for field, replacement in [("on_death", []), ("registered_events", [])]:
            changed = deepcopy(self.frozen)
            changed["records"][0][field] = replacement
            with self.assertRaises(ValueError):
                capture.qualify_capture(changed)
        changed = deepcopy(self.frozen)
        changed["records"][0]["on_death"][0]["registration_index"] = 0
        with self.assertRaises(ValueError):
            capture.qualify_capture(changed)

    def test_unknown_registration_cannot_silently_become_an_empty_vector(self):
        parent = b'monster.events = {"UnreviewedDeath"}\nmType:register(monster)\n'
        with self.assertRaisesRegex(ValueError, "unproved source event"):
            capture.resolve_parent(parent, {}, {"RottenBloodBossDeath"})

    def test_absence_proof_cannot_override_a_present_callback(self):
        parent = b'monster.events = {"RottenBloodBossDeath"}\nmType:register(monster)\n'
        with self.assertRaisesRegex(ValueError, "actually defined"):
            capture.resolve_parent(parent, {"RottenBloodBossDeath": ("new.lua", b"")},
                                   {"RottenBloodBossDeath"})


if __name__ == "__main__":
    unittest.main()
