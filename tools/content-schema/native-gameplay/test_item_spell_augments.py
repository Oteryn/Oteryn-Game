import unittest
import xml.etree.ElementTree as ET
from item_spell_augments import parse_item_spell_augments


def item(kind, value="4", spell="fierce berserk"):
    return ET.fromstring(f'<item><attribute key="augments" value="1"><attribute key="{spell}" value="{kind}"><attribute key="value" value="{value}"/></attribute></attribute></item>')


class ItemSpellAugments(unittest.TestCase):
    def test_pinned_sanguine_blade_units(self):
        # Canary 47dfd51f items.xml 43864; Crystal 201189f items.xml 43864.
        canary = parse_item_spell_augments(item("base damage"))
        crystal = parse_item_spell_augments(item("base", "400"), "crystal")
        self.assertEqual(canary, [{"spell": "fierce berserk", "kind": "base_damage", "value": 400}])
        self.assertEqual(canary[0], crystal[0])
        self.assertEqual(crystal[1]["kind"], "base_healing")

    def test_all_numeric_stages_and_cooldown_units(self):
        for kind in ("base healing", "critical hit chance", "critical extra damage", "mana leech", "life leech"):
            self.assertEqual(parse_item_spell_augments(item(kind, "8"))[0]["value"], 800)
        self.assertEqual(parse_item_spell_augments(item("cooldown", "2000"))[0]["value"], 2000)
        self.assertEqual(parse_item_spell_augments(item("ManaLeech", "800"), "crystal")[0]["value"], 800)

    def test_absence_disabled_unknown_and_bounds(self):
        self.assertEqual(parse_item_spell_augments(item("cooldown", "86400000"))[0]["value"], 86400000)
        node = item("base damage", "1000", "a" * 128)
        node[0].extend([node[0][0]] * 15)
        self.assertEqual(len(parse_item_spell_augments(node)), 16)
        self.assertEqual(parse_item_spell_augments(ET.fromstring("<item/>")), [])
        disabled = item("unsupported")
        disabled[0].set("value", "0")
        self.assertEqual(parse_item_spell_augments(disabled), [])
        for node in (item("wheel"), item("life leech", "-1"), item("base damage", "1001"), item("cooldown", "86400001")):
            with self.assertRaises(ValueError):
                parse_item_spell_augments(node)
        node = item("base damage")
        node[0].extend([node[0][0]] * 16)
        with self.assertRaises(ValueError):
            parse_item_spell_augments(node)

if __name__ == "__main__":
    unittest.main()
