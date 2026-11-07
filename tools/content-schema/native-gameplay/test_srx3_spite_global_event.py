"""Registered GlobalEvent identity must never masquerade as an actor callback."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location("spite_global", Path(__file__).with_name("build_srx3_spite_global_event.py"))
TOOL = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(TOOL)


class SpiteGlobalEventProof(unittest.TestCase):
    def test_one_registered_global_event_keeps_literal_source_and_no_actor_requirement(self):
        capture = json.loads(TOOL.OUTPUT.read_bytes())
        self.assertEqual(capture["revision"], TOOL.PIN)
        self.assertEqual(capture["schema"], "OTERYN_SOURCE_GLOBAL_EVENT/v1")
        self.assertEqual(len(capture["events"]), 1)
        event = capture["events"][0]
        self.assertEqual(event["name"], "CreateGoshnarSpiteFire")
        self.assertEqual(event["owner"], "RegisteredGlobalEvent")
        self.assertEqual(event["encounter"], {"family": "Encounter", "key": "oteryn:encounter.soul_war_taint_zones", "revision": "definition-r1"})
        self.assertEqual(event["item"], {"family": "Item", "key": "oteryn:item.tibia.i33877", "revision": "definition-r1"})
        self.assertEqual(event["positions"], [[33743, 31628, 14], [33736, 31632, 14], [33750, 31632, 14], [33742, 31637, 14]])
        self.assertEqual((event["interval_ms"], event["removal_delay_ms"], event["expiry_defense_add"]), (14000, 5000, 10))
        self.assertFalse(event["requires_parent_creature"])
        self.assertFalse(event["requires_committed_lever"])
        self.assertEqual(event["create_lines"], [417, 429])
        self.assertEqual(event["registration_lines"], [431, 432])

    def test_actual_encounter_has_registered_loop_without_boss_birth_trigger(self):
        root = Path(__file__).resolve().parents[3]
        document = json.loads((root / "content/encounters/definitions/encounters-00000-00107.json").read_bytes())
        row = next(row for row in document["records"] if row["declaration"]["identity"]["key"] == "oteryn:encounter.soul_war_taint_zones")
        details = row["authoring"]["profile"]["details"]
        rule = next((rule for rule in details["rules"] if rule["key"] == "spite_registered_global_fire"), None)
        self.assertIsNotNone(rule, "actual Encounter lacks its registered source GlobalEvent loop")
        self.assertEqual(rule["trigger"], {"kind": "timer_elapsed", "timer": "spite_global_fire"})
        timer = next(timer for timer in details["state"]["timers"] if timer["name"] == "spite_global_fire")
        self.assertEqual(timer, {"name": "spite_global_fire", "duration_ms": {"min": 14000, "max": 14000}, "repeat": True})
        branches = rule["actions"][0]["branches"]
        self.assertEqual([branch["weight"] for branch in branches], [1, 1, 1, 1])
        self.assertTrue(all(branch["actions"][0]["item"]["key"] == "oteryn:item.tibia.i33877" for branch in branches))
        self.assertTrue(all("revert_after_ms" not in branch["actions"][0] for branch in branches), "source expiry is a defense callback, not automatic Item decay")

    def test_changed_or_truncated_pinned_global_event_script_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / next(iter(TOOL.SOURCES))
            path.parent.mkdir(parents=True)
            path.write_text('GlobalEvent("CreateGoshnarSpiteFire")\n')
            with self.assertRaisesRegex(ValueError, "pinned Crystal GlobalEvent source changed"):
                TOOL.build(directory)


if __name__ == "__main__":
    unittest.main()
