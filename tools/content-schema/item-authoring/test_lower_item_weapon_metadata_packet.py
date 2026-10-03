import copy
import json
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator
from lower_item_weapon_metadata_packet import (
    FIELDS,
    PROOF,
    ROOT,
    build,
    check_parent_receipt,
    latest_frame,
    parse_value,
    qualify,
)
from weapon_metadata_source_aliases import build_alias_catalog, formal_weapon


class WeaponMetadata(unittest.TestCase):
    def test_exact_units_no_defaults_and_legacy_decoder_preserved(self):
        self.assertEqual(
            parse_value("hit_chance", "90%"), {"numerator": 90, "denominator": 1}
        )
        self.assertEqual(
            parse_value("hit_chance", "0%"), {"numerator": 0, "denominator": 1}
        )
        for value in ("90", "-1%", "101%", "true", ""):
            with self.assertRaises(ValueError):
                parse_value("hit_chance", value)
        for value in (str(-(2**31)), str(2**31 - 1)):
            self.assertEqual(parse_value("atk_mod", value), int(value))
        for value in (str(2**31), str(-(2**31) - 1), "true", "2.0"):
            with self.assertRaises(ValueError):
                parse_value("atk_mod", value)
        author = {
            "weapon_attack_modifier_points": -2,
            "weapon_absolute_hit_chance_percent": parse_value("hit_chance", "90%"),
        }
        self.assertEqual(
            formal_weapon(author),
            {
                "attack_modifier": -2,
                "hit_chance_percent": {"numerator": 90, "denominator": 1},
            },
        )
        schema = json.loads((Path(__file__).parent / "item.schema.json").read_text())
        weapon_schema = schema["properties"]["weapon"] | {"$defs": schema["$defs"]}
        self.assertEqual(
            list(
                Draft202012Validator(weapon_schema).iter_errors(
                    {"weapon_type": "distance_launcher", **formal_weapon(author)}
                )
            ),
            [],
        )
        self.assertEqual(formal_weapon({}), {})
        self.assertEqual(
            {r["source_field"] for r in build_alias_catalog()["fields"]}, set(FIELDS)
        )
        self.assertNotIn("hit_mod", FIELDS)

    def test_closed103_source_and_preserved_native_unknown_names(self):
        packet = build()
        self.assertEqual(
            packet["counts"],
            {
                "fields": 103,
                "items": 103,
                "attack_modifier": 78,
                "absolute_hit_percent": 25,
            },
        )
        self.assertEqual(len({r["target"]["key"] for r in packet["promotions"]}), 103)
        proof = json.loads((ROOT / PROOF).read_text())
        self.assertEqual(
            sorted(
                r["source_item_id"]
                for r in proof["records"]
                if r["native_name_guard"]["state"] == "UNKNOWN"
            ),
            [53225, 53226, 53227, 53228],
        )
        self.assertFalse(
            {8024, 8025}.intersection(r["source_item_id"] for r in proof["records"])
        )

    def test_latest_admitted_and_only_explicit_parent_hit_deltas(self):
        proof = json.loads((ROOT / PROOF).read_text())
        index = json.loads((ROOT / proof["official_current"]["index_path"]).read_text())
        self.assertEqual(latest_frame(index, proof), "client-15.30")
        for key, value in (
            ("newest", "client-15.40"),
            ("files", list(reversed(index["files"]))),
        ):
            bad = copy.deepcopy(index)
            bad[key] = value
            with self.assertRaises(ValueError):
                latest_frame(bad, proof)
        for lane in ("value", "native_weapon_guard", "native_name_guard"):
            bad = copy.deepcopy(proof)
            bad["records"][0][lane] = 999
            with self.assertRaises(ValueError):
                check_parent_receipt(ROOT, bad)
        bad = copy.deepcopy(proof)
        bad["records"][-1] = bad["records"][0]
        with self.assertRaises(ValueError):
            check_parent_receipt(ROOT, bad)

        # Python bool/int equality must not hide a malformed Native ratio.
        bad = copy.deepcopy(proof)
        source = next(
            s
            for s in bad["records"]
            if s["native_weapon_guard"]["value"]["hit_chance"]
            .get("value", {})
            .get("numerator")
            == 1
        )
        source["native_weapon_guard"]["value"]["hit_chance"]["value"]["numerator"] = (
            True
        )
        with self.assertRaisesRegex(ValueError, "native weapon projection"):
            check_parent_receipt(ROOT, bad)

    def test_late_own_source_opposition_and_existing_owner_conflicts(self):
        target = {
            "family": "Item",
            "key": "oteryn:item.tibia.i1",
            "revision": "definition-r1",
        }
        binding = {"target": target, "external_id": "1"}
        weapon = {
            "state": "KNOWN",
            "value": {
                "weapon_type": {"state": "KNOWN", "value": "DISTANCE"},
                "attack": {"state": "KNOWN", "value": 1},
            },
        }
        header = {
            "kind": "Item",
            "stack_class": "Unknown",
            "materializable": False,
            "client_projection": "ClientSafe",
        }
        name = {"state": "UNKNOWN"}
        source = {
            "source_item_id": 1,
            "target": target,
            "binding": binding,
            "field": "atk_mod",
            "value": 2,
            "official_name": "bow",
            "native_weapon_guard": weapon,
            "native_class_guard": header,
            "native_name_guard": name,
        }
        record = {
            "definition": {
                **copy.deepcopy(header),
                "identity": target,
                "semantics": {"weapon": copy.deepcopy(weapon)},
            }
        }
        obj = {"name": "bow", "flags": {"flags.take": True}}
        box = {
            "balanced": True,
            "inside_comment": False,
            "positive_exact_infobox_object_match": True,
        }
        params = {
            "itemid": ["1"],
            "actualname": ["bow"],
            "primarytype": ["Distance Weapons"],
            "atk_mod": ["2"],
        }
        own = [({}, box, params)]
        self.assertEqual(
            qualify(source, record, binding, obj, own, set())["facts"],
            {"weapon_attack_modifier_points": 2},
        )
        stack_successor = copy.deepcopy(record)
        stack_successor["definition"]["semantics"]["stack"] = {
            "state": "KNOWN",
            "value": {
                "stack_max": {"state": "UNKNOWN"},
                "stackable": {"state": "KNOWN", "value": False},
            },
        }
        self.assertEqual(
            qualify(source, stack_successor, binding, obj, own, set())["facts"],
            {"weapon_attack_modifier_points": 2},
        )
        for attack in (2, True):
            bad_record = copy.deepcopy(record)
            bad_record["definition"]["semantics"]["weapon"]["value"]["attack"][
                "value"
            ] = attack
            with self.assertRaisesRegex(ValueError, "native weapon type/projection"):
                qualify(source, bad_record, binding, obj, own, set())
        for key, value in (("kind", "World"), ("materializable", 0)):
            bad_record = copy.deepcopy(record)
            bad_record["definition"][key] = value
            with self.assertRaisesRegex(ValueError, "class/admission/projection"):
                qualify(source, bad_record, binding, obj, own, set())
        labelled = copy.deepcopy(params)
        labelled["name"] = ["Bow (Heavily Charged)"]
        self.assertEqual(
            qualify(source, record, binding, obj, [({}, box, labelled)], set())[
                "facts"
            ],
            {"weapon_attack_modifier_points": 2},
        )
        fallback = copy.deepcopy(params)
        fallback.pop("actualname")
        fallback["name"] = ["bow"]
        self.assertEqual(
            qualify(source, record, binding, obj, [({}, box, fallback)], set())[
                "facts"
            ],
            {"weapon_attack_modifier_points": 2},
        )
        for names in ([""], ["  "], ["bow", "bow"], ["different"]):
            bad = copy.deepcopy(labelled)
            bad["actualname"] = names
            bad["name"] = ["bow"]
            with self.assertRaises(ValueError):
                qualify(source, record, binding, obj, [({}, box, bad)], set())
        for field, value in (
            ("atk_mod", ["3"]),
            ("itemid", ["1, 2"]),
            ("actualname", ["other"]),
            ("primarytype", ["Quest Items"]),
        ):
            bad = copy.deepcopy(params)
            bad[field] = value
            with self.assertRaises(ValueError):
                qualify(source, record, binding, obj, own + [({}, box, bad)], set())
        record["authoring"] = {"weapon_attack_modifier_points": 3}
        with self.assertRaises(ValueError):
            qualify(source, record, binding, obj, own, set())
        source["value"] = True
        with self.assertRaises(ValueError):
            qualify(source, record, binding, obj, own, set())

    def test_pre_overlay_header_drift_still_fails_closed(self):
        # D316/D341: the guard reads the pre-overlay stage; a real header drift there must fail.
        import lower_item_weapon_metadata_packet as lane

        original = lane.pre_overlay_definitions
        key = json.loads(lane.OUTPUT.read_text())["promotions"][0]["target"]["key"]

        def drifted(root, current):
            definitions = original(root, current)
            row = definitions[key]
            definitions[key] = row | {"materializable": not row["materializable"]}
            return definitions

        lane.pre_overlay_definitions = drifted
        try:
            with self.assertRaisesRegex(ValueError, "class/admission/projection drift"):
                lane.build()
        finally:
            lane.pre_overlay_definitions = original


if __name__ == "__main__":
    unittest.main()
