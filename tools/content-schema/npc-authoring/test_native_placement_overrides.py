"""Native scheduled placements replace a single-cell migration hypothesis explicitly."""
import copy
import unittest
import convert_placements as producer


class NativePlacementOverrides(unittest.TestCase):
    def fixture(self):
        key = "oteryn:npc.test_trader"
        candidates = {key: {"placements": [{"position": {"x": 1, "y": 2, "z": 7}, "direction": "NORTH"}]}}
        override = {
            "basis": "Pinned startup schedule, with UTC selected by the existing runtime owner.",
            "source": {"origin": "canary", "revision": "4" * 40,
                       "path": "data/scripts/scheduled_trader.lua", "sha256": "a" * 64},
            "placements": [{"position": {"x": 10, "y": 20, "z": 6}, "direction": "SOUTH"},
                           {"position": {"x": 30, "y": 40, "z": 7}, "direction": "SOUTH"}],
        }
        return key, candidates, {"npcs": [key], "placement_overrides": {key: override}}

    def test_replaces_only_explicit_key_and_preserves_original_evidence(self):
        key, candidates, scope = self.fixture()
        original = copy.deepcopy(candidates)
        actual = producer.apply_placement_overrides(candidates, scope)
        self.assertEqual(candidates, original)
        self.assertEqual(actual[key]["placements"], scope["placement_overrides"][key]["placements"])
        evidence = producer.provenance(actual[key], actual[key]["placements"][0])
        self.assertEqual(evidence[0]["source_row"], "data/scripts/scheduled_trader.lua")
        self.assertEqual(evidence[0]["source_sha256"], "a" * 64)
        self.assertIn("4" * 40, evidence[0]["url"])
        self.assertEqual(producer.apply_placement_overrides(candidates, {"npcs": [key]}), original)

    def test_refuses_unbounded_ambiguous_or_unpinned_overrides(self):
        for case in range(9):
            key, candidates, scope = self.fixture()
            row = scope["placement_overrides"][key]
            if case == 0: scope["npcs"] = []
            elif case == 1: candidates = {}
            elif case == 2: row["placements"] = []
            elif case == 3: row["placements"] *= 9
            elif case == 4: row["placements"].append(copy.deepcopy(row["placements"][0]))
            elif case == 5: row["placements"][0]["direction"] = "UNKNOWN"
            elif case == 6: row["source"]["revision"] = "main"
            elif case == 7: row["source"]["sha256"] = "unknown"
            else: row["placements"][0]["position"]["z"] = 16
            with self.subTest(case=case), self.assertRaises(ValueError):
                producer.apply_placement_overrides(candidates, scope)


if __name__ == "__main__":
    unittest.main()
