"""C12's production item bonus plus selected weapon proficiency perks."""
import json
from decimal import Decimal
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[3]
ITEM = "oteryn:item.tibia.i50271"


def records(path):
    return json.loads((ROOT / path).read_bytes())["records"]


class C12ProficiencyMagnitude(unittest.TestCase):
    def test_greater_tiger_clash_uses_only_selected_perks(self):
        binding = next(row for row in records("content/proficiencies/bindings.json")
                       if row["item"]["key"] == ITEM)
        profile = next(row["definition"] for row in records(
            "content/proficiencies/proficiencies-00300-00442.json")
            if row["definition"]["identity"]["key"] == binding["profile_binding"]["key"])
        item = next(row for row in records("content/items/definitions/spell-native-profiles.json")
                    if row["production_definition"]["production_key"] == ITEM)
        flat = sum(augment["value"] for augment in item["attributes"]["spell_augments"]
                   if augment["kind"] == "base_damage" and augment["spell"] == "greater tiger clash")
        perks = {
            level["level"]: int(Decimal(str(perk["value"])) * 10_000)
            for level in profile["levels"] for perk in level["perks"]
            if perk["kind"] == "spell_augment" and perk["spell_client_id"] == 292
            and perk["augment"] == "base_damage"
        }
        self.assertEqual(perks, {1: 400, 3: 400})
        for selected, expected in [((), 100), ((1,), 104), ((3,), 104), ((1, 3), 108)]:
            with self.subTest(selected=selected):
                self.assertEqual(100 * (10_000 + flat + sum(perks[i] for i in selected)) // 10_000,
                                 expected)
