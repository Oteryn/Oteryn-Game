import copy
import unittest

from lower_item_weapon_metadata_packet import FIELDS, build, parse_value, qualify
from weapon_metadata_source_aliases import build_alias_catalog, formal_weapon


class WeaponMetadata(unittest.TestCase):
    def test_exact_units_no_defaults_and_legacy_decoder_preserved(self):
        self.assertEqual(parse_value("hit_chance", "90%"), {"numerator": 90, "denominator": 1})
        self.assertEqual(parse_value("hit_chance", "0%"), {"numerator": 0, "denominator": 1})
        for value in ("90", "-1%", "101%", "true", ""):
            with self.assertRaises(ValueError):
                parse_value("hit_chance", value)
        for value in (str(-(2**31)), str(2**31 - 1)):
            self.assertEqual(parse_value("atk_mod", value), int(value))
        for value in (str(2**31), str(-(2**31) - 1), "true", "2.0"):
            with self.assertRaises(ValueError):
                parse_value("atk_mod", value)
        author = {"weapon_attack_modifier_points": -2, "weapon_absolute_hit_chance_percent": parse_value("hit_chance", "90%")}
        self.assertEqual(formal_weapon(author), {"attack_modifier": -2, "hit_chance_percent": {"numerator": 90, "denominator": 1}})
        self.assertEqual(formal_weapon({}), {})
        self.assertEqual({r["source_field"] for r in build_alias_catalog()["fields"]}, set(FIELDS))
        self.assertNotIn("hit_mod", FIELDS)

    def test_closed79_source_and_preserved_native_unknown_names(self):
        packet = build()
        self.assertEqual(packet["counts"], {"fields": 79, "items": 79, "attack_modifier": 54, "absolute_hit_percent": 25})
        self.assertEqual(len({r["target"]["key"] for r in packet["promotions"]}), 79)

    def test_late_own_source_opposition_and_existing_owner_conflicts(self):
        target = {"family": "Item", "key": "oteryn:item.tibia.i1", "revision": "definition-r1"}
        binding = {"target": target, "external_id": "1"}
        weapon = {"state": "KNOWN", "value": {"weapon_type": {"state": "KNOWN", "value": "DISTANCE"}}}
        name = {"state": "UNKNOWN"}
        source = {"source_item_id": 1, "target": target, "binding": binding, "field": "atk_mod", "value": 2,
                  "official_name": "bow", "native_weapon_guard": weapon, "native_name_guard": name}
        record = {"definition": {"identity": target, "semantics": {"weapon": weapon}}}
        obj = {"name": "bow", "flags": {"flags.take": True}}
        box = {"balanced": True, "inside_comment": False, "positive_exact_infobox_object_match": True}
        params = {"itemid": ["1"], "actualname": ["bow"], "primarytype": ["Distance Weapons"], "atk_mod": ["2"]}
        own = [({}, box, params)]
        self.assertEqual(qualify(source, record, binding, obj, own, set())["facts"], {"weapon_attack_modifier_points": 2})
        for field, value in (("atk_mod", ["3"]), ("itemid", ["1, 2"]), ("actualname", ["other"]), ("primarytype", ["Quest Items"])):
            bad = copy.deepcopy(params); bad[field] = value
            with self.assertRaises(ValueError):
                qualify(source, record, binding, obj, own + [({}, box, bad)], set())
        record["authoring"] = {"weapon_attack_modifier_points": 3}
        with self.assertRaises(ValueError):
            qualify(source, record, binding, obj, own, set())


if __name__ == "__main__":
    unittest.main()
