import importlib.util
import unittest
from pathlib import Path

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

    def test_missing_attacks_are_not_zero(self):
        rows = {row["name"]: row for row in power_score._creature_rows()}
        self.assertEqual(rows["1st Mate Ratticus"]["data_quality"], "missing_attacks")
        self.assertIsNone(rows["1st Mate Ratticus"]["threat"])
        self.assertGreater(rows["Bulltaur Brute"]["dps"], 0)

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
