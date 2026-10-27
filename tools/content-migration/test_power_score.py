import importlib.util
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

PATH = Path(__file__).with_name("power_score.py")
SPEC = importlib.util.spec_from_file_location("power_score", PATH)
power_score = importlib.util.module_from_spec(SPEC)
assert SPEC.loader
SPEC.loader.exec_module(power_score)


class PowerScoreTests(unittest.TestCase):
    def test_player_might_grows_and_bis_improves_it(self):
        low = power_score.player_might("knight", 50, "normal")
        high = power_score.player_might("knight", 100, "normal")
        bis = power_score.player_might("knight", 100, "bis")
        self.assertGreater(high["might"], low["might"])
        self.assertGreater(bis["might"], high["might"])

    def test_authored_graph_recovers_import_gaps(self):
        rows = {row["name"]: row for row in power_score._creature_rows()}
        for name in ("1st Mate Ratticus", "Hydra", "Bulltaur Brute"):
            self.assertEqual(rows[name]["data_quality"], "complete")
            self.assertGreater(rows[name]["dps"], 0)
            self.assertIsNotNone(rows[name]["threat"])
        self.assertTrue(all(a["source"]["formula"] for a in rows["Hydra"]["attacks"]
                            if a["classification"] == "damage"))

    def test_missing_attack_fallback_stays_null_even_with_known_damage(self):
        ability = {"definition": {"effects": [{"key": "effect"}]}}
        effect = {"definition": {"effect_family": "Damage", "formula": {"key": "formula"}}}
        parsed = power_score._resolve_attack("attack", {"interval_ms": 2000, "chance_ppm": 1000000},
                                             {"attack": (ability, "abilities.json")},
                                             {"effect": (effect, "effects.json")}, {})
        self.assertEqual(parsed[0]["classification"], "unresolved")
        self.assertEqual(parsed[0]["reason"], "damage_without_numbers")
        original_index = power_score._graph_index
        def without_hydra_melee_formula(folder):
            records = original_index(folder)
            if folder == "content/abilities/formulas":
                records.pop("oteryn:formula.creature.hydra.attack-1-1")
            return records
        with patch.object(power_score, "_graph_index", side_effect=without_hydra_melee_formula):
            rows = {row["name"]: row for row in power_score._creature_rows()}
        self.assertGreater(rows["Hydra"]["dps"], 0)
        self.assertEqual(rows["Hydra"]["data_quality"], "missing_attacks")
        self.assertIsNone(rows["Hydra"]["threat"])

    def test_graph_classifies_non_damage_and_evaluates_numeric_formulas(self):
        ability = {"definition": {"effects": [{"key": "effect"}]}}
        def resolve(family, formula, schedule=None):
            effect = {"definition": {"effect_family": family, "formula": {"key": "formula"}}}
            return power_score._resolve_attack(
                "attack", schedule or {"interval_ms": 2000, "chance_ppm": 500000},
                {"attack": (ability, "abilities.json")}, {"effect": (effect, "effects.json")},
                {"formula": ({"authoring": {"profile": formula}}, "formulas.json")})[0]
        self.assertEqual(resolve("Heal", {})["classification"], "non_damage")
        for formula, maximum in (({"formula": "Range", "minimum": 0, "maximum": 400}, 400),
                                 ({"formula": "MeleeAttackSkill", "attack": 100, "skill": 70}, 400)):
            attack = resolve("Damage", formula)
            self.assertEqual(attack["maximum"], maximum)
            self.assertEqual(attack["dps"], 50)
        attack = resolve("Damage", {"formula": "CasterMagnitude"},
                         {"interval_ms": 1000, "chance_ppm": 1000000,
                          "magnitude": {"minimum": 10, "maximum": 20}})
        self.assertEqual(attack["dps"], 15)
        self.assertEqual(resolve("Damage", {"formula": "CasterMagnitude"})["classification"], "unresolved")

    def test_spawn_weighting_uses_points_floor_distance_and_uniform_fallback(self):
        def area(key, x=None):
            return {"identity": {"key": key}, **({"position": {"x": x, "y": 0, "floor": 7,
                    "coordinate_frame": "global-target-2026-09-27"}} if x is not None else {})}
        areas = [area("a", 0), area("b", 20), area("c"), area("d", 1000)]
        members = {a["identity"]["key"]: ["rat", "snake"] for a in areas}
        def point(creature, x, floor=7):
            return {"creature": creature, "cell": {"x": x, "y": 0, "floor": floor}}
        weights = power_score._spawn_weights(areas, members,
                    [point("rat", 1), point("rat", 2), point("snake", 3), point("rat", 19),
                     point("snake", 0, 8), point("rat", 500), point("unlisted", 0)])
        self.assertEqual(weights["a"], ({"rat": 2, "snake": 1}, "spawns"))
        self.assertEqual(weights["b"], ({"rat": 1}, "spawns"))
        for key in ("c", "d"):
            self.assertEqual(weights[key], ({"rat": 1, "snake": 1}, "uniform"))
        # Population proportions, rather than species count or raw map population,
        # determine the base Threat. AoE and pack pressure are disabled here.
        model = {**power_score._initial_model(), "pack_pressure": 0., "aoe_knight": 1.}
        creatures = [{"key": "rat", "threat": 10, "boss": False},
                     {"key": "snake", "threat": 40, "boss": False}]
        area_input = {"spawn_weights": weights["a"][0]}
        prediction = power_score._area_predictions([area_input], creatures, model)[0]
        self.assertEqual(prediction["threat"], 20)

    def test_holdout_metrics_and_documented_mae(self):
        data = json.loads(power_score.OUTPUT.read_text())
        calibration = data["calibration"]
        doc = (power_score.ROOT / "docs/reference/power-score.md").read_text()
        for split in ("train", "holdout", "overall"):
            metric = calibration["metrics"][split]
            self.assertGreater(metric["observations"], 0)
            for vocation, row in [("all", metric), *metric["per_vocation"].items()]:
                expected = f"| {split} | {vocation} | {row['observations']} | {row['mean_absolute_error_levels']} | {row['median_absolute_error_levels']} |"
                self.assertIn(expected, doc)
        self.assertIn(f"overall MAE **{calibration['mean_absolute_error_levels']} levels**", doc)
        self.assertLess(calibration["metrics"]["holdout"]["median_absolute_error_levels"], 100)
        self.assertEqual(calibration["observations"], sum(calibration["metrics"][s]["observations"] for s in ("train", "holdout")))
        for i, area in enumerate(data["areas"]):
            self.assertEqual(area["split"], "holdout" if (i + 1) % 5 == 0 else "train")

    def test_fit_never_reads_holdout_recommendations(self):
        areas = [{"split": split, "spawn_weights": {"rat": 1},
                  "source_recommended_levels": {"knight": level, "paladin": level, "mage": level}}
                 for split, level in (("train", 100), ("holdout", 200))]
        creatures = [{"key": "rat", "hp": 1000, "armor": 0, "defence": 0,
                      "dps": 50, "data_quality": "complete", "boss": False}]
        before = power_score._fit(areas, creatures)
        areas[1]["source_recommended_levels"] = {"knight": 1, "paladin": 2000, "mage": 5000}
        self.assertEqual(before, power_score._fit(areas, creatures))

    def test_metrics_count_above_range_predictions_and_exclude_unknown(self):
        area = {"split": "holdout", "threat": 10000,
                "source_recommended_levels": {"knight": 100, "paladin": 100, "mage": 100},
                "green_level": {v: {"normal": None} for v in power_score.VOCATIONS}}
        result = power_score._metrics([area, {**area, "threat": None}])["holdout"]
        self.assertEqual(result["observations"], 4)
        self.assertEqual(result["mean_absolute_error_levels"], 1901)
        self.assertEqual(result["median_absolute_error_levels"], 1901)

    def test_regeneration_is_byte_identical_across_processes(self):
        with tempfile.TemporaryDirectory(dir=power_score.ROOT) as directory:
            paths = [Path(directory) / f"run-{i}.json" for i in range(2)]
            for path in paths:
                subprocess.run(["python3", str(PATH), "--output", str(path)], check=True)
            self.assertEqual(paths[0].read_bytes(), paths[1].read_bytes())
            self.assertEqual(paths[0].read_bytes(), power_score.OUTPUT.read_bytes())
            data = json.loads(paths[0].read_text())
            for key in ("areas", "creatures"):
                self.assertEqual([r["key"] for r in data[key]], sorted(r["key"] for r in data[key]))
            records = [line for line in paths[0].read_text().splitlines() if line.startswith("    {\"")]
            self.assertEqual(len(records), len(data["areas"]) + len(data["creatures"]))

    def test_encounter_reports_times_and_pack_pressure(self):
        player = power_score.player_might("knight", 100)
        creature = {"ehp": 1000, "dps": 50, "threat": 22}
        solo = power_score.encounter(player, creature)
        pack = power_score.encounter(player, creature, pack_size=3)
        self.assertGreater(pack["time_to_kill_seconds"], solo["time_to_kill_seconds"])
        self.assertLess(pack["time_to_die_seconds"], solo["time_to_die_seconds"])

    def test_party_modes(self):
        self.assertEqual(power_score.party_might([20, 30], "lowest_member"), 20)
        self.assertGreater(power_score.party_might([20, 30]), 30)

    def test_verdict_boundaries(self):
        self.assertEqual(power_score.verdict(100, 25), "trivial")
        self.assertEqual(power_score.verdict(100, 60), "easy")
        self.assertEqual(power_score.verdict(100, 100), "fair")
        self.assertEqual(power_score.verdict(100, 160), "hard")
        self.assertEqual(power_score.verdict(100, 161), "deadly")


if __name__ == "__main__":
    unittest.main()
