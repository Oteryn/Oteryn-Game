"""Official news overrides preserve the base import and unrelated gameplay fields."""
import copy
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
LEDGER = ROOT / "content/creatures/official-balance.json"
STAGE = ROOT / "docs/agents/evidence/monster-server-import-20261002/creature-admission-stage.json"


class OfficialCreatureBalanceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.ledger = json.loads(LEDGER.read_bytes())
        cls.stage = json.loads(STAGE.read_bytes())

    def test_news_overrides_preserve_every_unrelated_profile_field(self):
        selected = {"oteryn:creature." + row["creature"] for row in self.ledger["overrides"]}
        before = {"authoring_profiles": [row for row in self.stage["authoring_profiles"] if row["target"]["key"] in selected]}
        after = {"authoring_profiles": json.loads((ROOT / "content/world/definitions/declarations.json").read_bytes())["authoring_profiles"]}
        after["authoring_profiles"] = [row for row in after["authoring_profiles"] if row["target"]["key"] in selected]
        restored = copy.deepcopy(after)
        profiles = {row["target"]["key"]: row["data"]["profile"]
                    for row in restored["authoring_profiles"] if row["target"]["family"] == "Creature"}
        # Undo in reverse news order, including the striker's two successive changes.
        for row in reversed(self.ledger["overrides"]):
            profile = profiles["oteryn:creature." + row["creature"]]
            self.assertIn(str(row["news_id"]), self.ledger["sources"])
            if row["field"] == "experience":
                self.assertEqual(profile["experience"], row["after"])
                profile["experience"] = row["before"]
            else:
                element = row["field"].removesuffix("_sensitivity")
                resistance = next(r for r in profile["resistances"] if r["damage_type"] == element)
                self.assertEqual(resistance["percent"], {"numerator": 100 - row["after"], "denominator": 1})
                resistance["percent"] = {"numerator": 100 - row["before"], "denominator": 1}
        # Admission predates the append-only Item key migration; the canonical
        # writer also omits empty immunity lists. Neither is a balance change.
        aliases = {row["key"]: row["target"] for row in json.loads(
            (ROOT / "content/items/aliases.json").read_bytes())["entries"] if row["state"] == "ALIAS"}

        def canonical(value):
            if isinstance(value, str):
                return aliases.get(value, value)
            if isinstance(value, list):
                return [canonical(child) for child in value]
            if isinstance(value, dict):
                return {key: canonical(child) for key, child in value.items()
                        if not (key == "immunities" and child == [])}
            return value

        self.assertEqual(
            {row["target"]["key"]: row for row in restored["authoring_profiles"]},
            canonical({row["target"]["key"]: row for row in before["authoring_profiles"]}),
        )

    def test_percentage_xp_uses_nearest_integer_on_base_values(self):
        for row in self.ledger["overrides"]:
            name = row["creature"]
            if row["field"] != "experience" or row["news_id"] != 8935:
                continue
            factor = 118 if name.startswith("devoted_radiant_") else 113 if name.startswith("radiant_") else 93
            self.assertEqual(row["after"], (row["before"] * factor + 50) // 100)

    def test_runtime_native_profiles_have_the_final_news_values(self):
        document = json.loads((ROOT / "content/creatures/definitions/spell-native-profiles.json").read_bytes())
        profiles = {row["profile"]["target"]["key"]: row["profile"]["data"]["profile"]
                    for row in document["records"]}
        final = {(row["creature"], row["field"]): row["after"] for row in self.ledger["overrides"]}
        for (creature, field), expected in final.items():
            profile = profiles["oteryn:creature." + creature]
            if field == "experience":
                self.assertEqual(profile[field], expected, creature)
            else:
                element = field.removesuffix("_sensitivity")
                resistance = next(row for row in profile["resistances"] if row["damage_type"] == element)
                self.assertEqual(resistance["percent"], {"numerator": 100 - expected, "denominator": 1}, creature)

    def test_no_gold_coin_tables_are_invented(self):
        reference = json.loads((ROOT / "content/world/definitions/reference.json").read_bytes())
        loot = {row["identity"]["key"]: row for row in reference["records"] if row["kind"] == "Loot"}
        self.assertEqual(len(self.ledger["gold_gain"]), 18)
        for row in self.ledger["gold_gain"]:
            entries = loot["oteryn:loot.creature." + row["creature"]]["entries"]
            self.assertFalse(any(entry["item"]["key"] == "oteryn:item.tibia.i3031" for entry in entries))
            self.assertEqual(row["status"], "not applied (no explicit gold)")


if __name__ == "__main__":
    unittest.main()
