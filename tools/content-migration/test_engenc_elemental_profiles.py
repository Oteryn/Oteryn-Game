import copy
import json
import unittest
from pathlib import Path

import engenc_elemental_profiles as elemental


class ElementalProfiles(unittest.TestCase):
    def test_exact_two_zero_xp_nonprey_policies_preserve_existing_profiles(self):
        root = Path(__file__).resolve().parents[2]
        document = json.loads(
            (
                root / "content/creatures/definitions/spell-native-profiles.json"
            ).read_text()
        )
        document["records"] = [
            r
            for r in document["records"]
            if r["profile"]["target"]["key"]
            not in {
                "oteryn:creature.greater_fire_elemental",
                "oteryn:creature.greater_energy_elemental",
            }
        ]
        before = copy.deepcopy(document["records"])
        elemental.apply(document)
        self.assertEqual(document["records"][:-2], before)
        for row, (source, target, name) in zip(
            document["records"][-2:], elemental.PAIRS
        ):
            original = next(
                r
                for r in before
                if r["profile"]["target"]["key"] == "oteryn:creature." + source
            )
            self.assertEqual(
                row["profile"]["target"]["key"], "oteryn:creature." + target
            )
            self.assertEqual(row["presentation"], original["presentation"])
            profile = row["profile"]["data"]["profile"]
            self.assertEqual(
                profile["health"], original["profile"]["data"]["profile"]["health"]
            )
            self.assertEqual(
                profile["resistances"],
                original["profile"]["data"]["profile"]["resistances"],
            )
            self.assertEqual(profile["experience"], 0)
            self.assertEqual(profile["details"]["display_name"], name)
            self.assertFalse(any(profile["details"]["system_eligibility"].values()))
            self.assertNotIn("soul_core_item", profile["details"])
            self.assertNotIn("corpse_item", profile["details"])
            closure = row["attack_definitions"]
            self.assertIn(row["profile"], closure["profiles"])
            creatures = [
                r for r in closure["records"] if r["identity"]["family"] == "Creature"
            ]
            self.assertEqual(len(creatures), 1)
            self.assertEqual(creatures[0]["identity"], row["profile"]["target"])
            self.assertIsNone(creatures[0]["loot"])

        loot = json.loads(
            (root / "tools/content-schema/native-gameplay/loot-tables.json").read_text()
        )
        for _, target, _ in elemental.PAIRS:
            matches = [
                r
                for r in loot["creature_loot"]
                if r["creature"]["key"] == "oteryn:creature." + target
            ]
            self.assertEqual(len(matches), 1)
            self.assertIsNone(matches[0]["loot"])
        with self.assertRaisesRegex(ValueError, "duplicate authored elemental"):
            elemental.apply(document)
