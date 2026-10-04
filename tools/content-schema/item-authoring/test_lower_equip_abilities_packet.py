"""No-network checks for `lower_equip_abilities_packet.py` (EQUIP-CONTENT-1).

Synthetic snapshots and Canary attribute maps cover the key mapping, the holds, wiki precedence,
the speed-unit check, the timed flag and the source-free facts packet; the committed files must
rebuild byte for byte. Runs as `python3 test_lower_equip_abilities_packet.py` or under
`python3 -m unittest <path>`.
"""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import lower_equip_abilities_packet as lower

KEY = "oteryn:item.tibia.i{}"
UNKNOWN = {"state": "UNKNOWN"}


def snapshot(records):
    return {
        "snapshot_sha256": "0" * 64,
        "records": {
            KEY.format(item_id): {
                "item_id": item_id,
                "observations": [
                    {"page_id": page_id, "revision_id": 1, "fields": fields}
                    for page_id, fields in pages
                ],
            }
            for item_id, pages in records.items()
        },
    }


def points(kind, value):
    return {
        "evaluation_phase": UNKNOWN,
        "kind": kind,
        "parameter": {
            "state": "KNOWN",
            "value": {"kind": "SIGNED_POINTS", "value": value},
        },
        "priority": UNKNOWN,
        "target_domain": UNKNOWN,
    }


def charged():
    return {
        "semantics": {
            "charges": {
                "state": "KNOWN",
                "value": {"count": {"state": "KNOWN", "value": 9}},
            }
        }
    }


# Every run needs one wiki/Canary speed pair to prove the unit.
SPEED_PAIR = {1: [(1, {"attrib": "speed +20"})]}
SPEED_CANARY = {1: {"speed": "20"}}


def sources(records, canary, definitions):
    ids = set(records) | set(canary)
    definitions = {KEY.format(1): {"semantics": {}}, **definitions}
    return lower.ability_sources(
        snapshot({**SPEED_PAIR, **records}),
        {**SPEED_CANARY, **canary},
        ids | {1},
        definitions,
    )


class CanaryMapping(unittest.TestCase):
    def test_modifiers(self):
        observed, typed = lower.canary_modifiers(
            {"skillfist": "6", "speed": "-10", "suppressdrown": "1", "weight": "80"}
        )
        self.assertEqual(
            observed, {"skillfist": "6", "speed": "-10", "suppressdrown": "1"}
        )
        self.assertEqual(
            [row["kind"] for row in typed], ["SKILL_FIST", "SPEED", "SUPPRESS_DROWN"]
        )
        self.assertEqual(typed[1], points("SPEED", -10))
        self.assertEqual(
            typed[2]["parameter"]["value"], {"kind": "BOOLEAN", "value": True}
        )
        self.assertEqual(lower.canary_modifiers({"weight": "80"}), (None, None))
        for attrs, verdict in (
            ({"skillsword": "0"}, "MALFORMED"),
            ({"skillsword": "x"}, "MALFORMED"),
            ({"suppressdrunk": "0"}, "MALFORMED"),
            ({"skillsword": "4", "manashield": "1"}, "UNMAPPED"),
            ({"healthgain": "2", "healthticks": "6000"}, "UNMAPPED"),
            ({"firemagiclevelpoints": "1"}, "UNMAPPED"),
            ({"skillfist": "1", "mantra": "3"}, "UNMAPPED"),
            ({"elementalbond": "5"}, "UNMAPPED"),
        ):
            self.assertEqual(lower.canary_modifiers(attrs)[1], verdict, attrs)

    def test_resistances(self):
        observed, typed = lower.canary_resistances(
            {
                "absorbpercentpoison": "5",
                "fieldabsorbpercentfire": "90",
                "absorbpercentphysical": "-3",
            }
        )
        self.assertEqual(len(observed), 3)
        self.assertEqual(
            [(row["kind"], row["percent"]["value"]["numerator"]) for row in typed],
            [("EARTH", 5), ("PHYSICAL", -3), ("FIRE_FIELD", 90)],
        )
        self.assertEqual(lower.canary_resistances({"armor": "2"}), (None, None))
        for attrs, verdict in (
            ({"absorbpercentfire": "101"}, "MALFORMED"),
            ({"absorbpercentfire": "0"}, "MALFORMED"),
            ({"absorbpercentearth": "5", "absorbpercentpoison": "6"}, "MALFORMED"),
            ({"absorbpercentmagic": "5"}, "UNMAPPED"),
        ):
            self.assertEqual(lower.canary_resistances(attrs)[1], verdict, attrs)


class Sources(unittest.TestCase):
    def test_wiki_first_and_fallback_where_silent(self):
        items, holds, pairs = sources(
            {2: [(2, {"attrib": "sword fighting +4"})], 3: [(3, {"weight": "1"})]},
            {2: {"skillsword": "4", "absorbpercentfire": "5"}, 3: {"skillclub": "2"}},
            {KEY.format(2): {"semantics": {}}, KEY.format(3): charged()},
        )
        self.assertEqual((holds, pairs), ([], 1))
        by_key = {item["item_key"]: item for item in items}
        # The wiki speaks on the modifiers of i2, so only its resistances take Canary.
        self.assertEqual(
            [row["field_path"] for row in by_key[KEY.format(2)]["fallback"]],
            ["protection.resistances"],
        )
        self.assertEqual(
            [source["class"] for source in by_key[KEY.format(2)]["sources"]],
            ["TIBIAWIKI", "OTS_HYPOTHESIS_ONLY"],
        )
        self.assertEqual(
            by_key[KEY.format(3)]["fallback"],
            [
                {
                    "field_path": "skill_modifiers.modifiers",
                    "value": [points("SKILL_CLUB", 2)],
                }
            ],
        )
        self.assertTrue(by_key[KEY.format(3)]["timed"])
        self.assertFalse(by_key[KEY.format(2)]["timed"])

    def test_holds(self):
        items, holds, _ = sources(
            {},
            {2: {"invisible": "1"}, 3: {"skillclub": "2"}},
            {KEY.format(2): {"semantics": {}}, KEY.format(3): {}},
        )
        self.assertEqual(
            [(hold["item_key"], hold["reason"]) for hold in holds],
            [
                (KEY.format(2), "UNMAPPED_CANARY_VALUE"),
                (KEY.format(3), "NO_CANONICAL_ITEM_SEMANTICS"),
            ],
        )
        # A held Item is still listed with its sources; an Item without semantics is not.
        self.assertEqual(
            [(item["item_key"], item["fallback"]) for item in items],
            [(KEY.format(1), []), (KEY.format(2), [])],
        )

    def test_speed_unit_disagreement_fails(self):
        with self.assertRaises(ValueError):
            sources({2: [(2, {"attrib": "speed +20"})]}, {2: {"speed": "40"}}, {})

    def test_unsourced_materialized_ability_fails(self):
        resisting = {
            "semantics": {
                "protection": {
                    "state": "KNOWN",
                    "value": {
                        "resistances": {
                            "state": "KNOWN",
                            "value": [{"kind": "FIRE"}],
                        }
                    },
                }
            }
        }
        with self.assertRaises(ValueError):
            sources({}, {}, {KEY.format(2): resisting})

    def test_facts_packet_names_no_source(self):
        facts, _ = lower.output_bytes(
            snapshot(SPEED_PAIR),
            {1, 3},
            {KEY.format(1): {"semantics": {}}, KEY.format(3): charged()},
            b"",
            canary={**SPEED_CANARY, 3: {"skillclub": "2"}},
        )
        text = facts.decode("utf-8").lower()
        for word in ("canary", "wiki", "skillclub", "source", "hypothesis"):
            self.assertNotIn(word, text)
        self.assertEqual(
            [
                (row["item_key"], row["field_path"])
                for row in json.loads(facts)["facts"]
            ],
            [(KEY.format(3), "skill_modifiers.modifiers")],
        )

    def test_committed_files_rebuild(self):
        self.assertEqual(lower.main(["--check"]), 0)


if __name__ == "__main__":
    unittest.main()
