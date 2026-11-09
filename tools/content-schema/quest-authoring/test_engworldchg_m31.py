"""Selected world-change journeys must have executable, source-bound personal stages."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[3]
PROFILE = ROOT / "content/quests/world-change-journals.json"
KEYS = {
    "fire_from_the_earth", "fury_gates", "grimvale", "hive_born", "hive_outpost",
    "horse_station", "insectoid_invasion", "jungle_camp", "kingsday", "lumberjack",
}


class NativeWorldChangeJournals(unittest.TestCase):
    def test_fire_has_a_native_fifty_kill_goal_and_achievement(self):
        program = next(p for p in json.loads(PROFILE.read_text())["programs"]
                       if p["key"] == "fire_from_the_earth")
        self.assertTrue(program["runtime_enabled"])
        self.assertEqual(program["achievement"], "oteryn:achievement/fire_from_the_earth")
        self.assertEqual(program["completion"]["counter_target"], 50)
        entry, = program["world_callbacks"]
        cohort = entry["effects"]["encounter"]
        self.assertEqual(len(cohort["monsters"]), 7)
        self.assertEqual({m["creature"] for m in cohort["monsters"]},
                         {c["creature"] for c in program["counters"]})
        self.assertEqual(cohort["players"][0]["from"], {"x":32037,"y":32588,"floor":-7})
        self.assertTrue(any(s["track"].endswith("/kills") and s["maximum"] == 49
                            and s.get("preserve") for s in entry["storage"]))

    def test_every_selected_recipe_has_exact_stage_bindings(self):
        document = json.loads(PROFILE.read_text())
        self.assertEqual(document["schema"], 1)
        self.assertEqual({p["key"] for p in document["programs"]}, KEYS)
        definitions = {}
        for path in (ROOT / "content/quests/definitions").glob("quests-*.json"):
            for row in json.loads(path.read_text())["records"]:
                definition = row["definition"]
                definitions[definition["identity"]["key"]] = definition
        for program in document["programs"]:
            with self.subTest(program=program["key"]):
                definition = definitions[program["quest"]]
                recipe = definition.get("recipe") or definition["oteryn_recipe"]["payload"]["recipe"]
                stages = recipe.get("stages", recipe.get("steps"))
                self.assertEqual(set(program["stage_bindings"]), {s["key"] for s in stages})
                self.assertEqual(program["requires_premium"], recipe["requirements"]["premium"])
                self.assertEqual(program["minimum_level"], recipe["requirements"]["min_level"])
                for callback in program["world_callbacks"]:
                    self.assertEqual(callback.get("minimum_level", 0), recipe["requirements"]["min_level"])
                self.assertEqual(program["repeat"], "once")
                self.assertEqual(program["mode"], "personal_on_demand")
                self.assertTrue(program["sources"])
                self.assertTrue(all(s.startswith("https://") for s in program["sources"]))
                self.assertTrue(program["world_callbacks"] or program["npc_callbacks"])
                for callback in program["world_callbacks"] + program["npc_callbacks"]:
                    self.assertEqual(callback["quest"], program["quest"])
                    self.assertFalse(callback.get("world_change"), "chosen on-demand stages cannot acquire calendar gates")
                    self.assertTrue(callback["storage"], "native occurrence must have a fenced personal phase")

    def test_source_object_bindings_are_specific_native_scenes(self):
        programs = {p["key"]: p for p in json.loads(PROFILE.read_text())["programs"]}
        expected = {
            "fury_gates": (33290, 31785, -13, 776),
            "grimvale": (33311, 31709, -7, 1866),
            "hive_outpost": (32159, 32844, -7, 3733),
        }
        for key, (x, y, floor, appearance) in expected.items():
            selectors = [c["selector"] for c in programs[key]["world_callbacks"]]
            self.assertTrue(any(s.get("position") == {"x": x, "y": y, "floor": floor}
                                and s.get("appearance") == appearance for s in selectors), key)
        self.assertEqual(programs["kingsday"]["achievement"], "oteryn:achievement/loyal_subject")
        self.assertEqual(programs["lumberjack"]["achievement"], "oteryn:achievement/whistle_blower")
        self.assertEqual(programs["fire_from_the_earth"]["achievement"], "oteryn:achievement/fire_from_the_earth")

    def test_grimvale_supplies_the_missing_wolves_and_stops_at_the_goal(self):
        program = next(p for p in json.loads(PROFILE.read_text())["programs"] if p["key"] == "grimvale")
        entry = next(c for c in program["world_callbacks"] if c["key"].endswith("/island"))
        cohort = entry["effects"]["encounter"]
        self.assertEqual(len(cohort["players"]), 1)
        self.assertEqual(len(cohort["monsters"]), 5)
        self.assertTrue(all(m["creature"] == "oteryn:creature.werewolf" for m in cohort["monsters"]))
        cells = {tuple(m["position"][k] for k in ("x", "y", "floor")) for m in cohort["monsters"]}
        cells.add(tuple(cohort["players"][0]["to"][k] for k in ("x", "y", "floor")))
        self.assertEqual(cells, {(x, y, -7) for x in range(33310, 33313) for y in (31710, 31711)})
        self.assertTrue(any(s["track"].endswith("/kills") and s["maximum"] == 9 and s.get("preserve") for s in entry["storage"]))
        self.assertTrue(any(s["track"].endswith("/waves") and s.get("increment") for s in entry["storage"]))
        self.assertEqual(program["counters"][0]["area"], [33300, 33360, 31665, 31730, 0, 15])

    def test_insectoid_uses_the_source_scout_pair_and_goal_receipt(self):
        program = next(p for p in json.loads(PROFILE.read_text())["programs"]
                       if p["key"] == "insectoid_invasion")
        self.assertTrue(program["runtime_enabled"])
        self.assertFalse(program["requires_premium"])
        entry, = program["world_callbacks"]
        self.assertEqual(entry["trigger"], "step_in")
        cohort = entry["effects"]["encounter"]
        self.assertEqual({tuple(m["position"][k] for k in ("x", "y", "floor"))
                          for m in cohort["monsters"]},
                         {(32208, 32058, -7), (32209, 32060, -7)})
        self.assertTrue(all(m["creature"] == "oteryn:creature.insectoid_scout"
                            for m in cohort["monsters"]))
        self.assertEqual(program["completion"], {
            "callbacks": [], "counter_target": 10,
            "counter_track": "oteryn:quest-progress/world-change-journal/insectoid_invasion/kills",
        })
        self.assertTrue(any(s["track"].endswith("/kills") and s["maximum"] == 9
                            and s.get("preserve") for s in entry["storage"]))


if __name__ == "__main__":
    unittest.main()
