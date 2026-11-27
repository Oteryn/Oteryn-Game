"""The published Item provider preserves independently qualified consumer facts."""

import json
import unittest

import native_item_consumer_facts as facts
from crystal_equipment_slice_4 import global_augments
from import_cyclopedia_global import VOCATIONS, load_capture


class NativeItemConsumerFacts(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.qualified = json.loads((facts.ROOT / facts.PATH).read_bytes())
        cls.profiles = facts.published_items(
            lambda path: (facts.ROOT / path).read_bytes()
        )

    def test_published_loot_keeps_legacy_market_eligibility(self):
        missing = [
            key
            for key in self.qualified["marketable_items"]
            if not facts.marketable(self.profiles[key])
        ]
        self.assertEqual(missing, [])

    def test_published_corpse_door_and_container_admissions_have_display_names(self):
        actual = {
            key: facts.display_name(self.profiles[key])
            for key in self.qualified["display_names"]
        }
        self.assertEqual(actual, self.qualified["display_names"])

    def test_newly_admitted_noncontainers_use_the_global_ware_id(self):
        for key, eligible in self.qualified["newly_admitted_marketability"].items():
            profile = self.profiles[key]
            with self.subTest(key=key):
                self.assertTrue(profile["admission"]["materializable"])
                self.assertEqual(facts.marketable(profile), eligible)

    def test_published_equipment_requirements_and_sockets_follow_cyclopedia(self):
        capture = load_capture(facts.ROOT)
        for identifier in (9020, 22127, 22129, 22130, 22132):
            with self.subTest(identifier=identifier):
                basic = capture[identifier]["basic"]
                equipment = self.profiles[f"{facts.PREFIX}{identifier}"]["semantics"][
                    "equipment"
                ]
                for pattern in equipment["value"]["patterns"]["value"]:
                    self.assertEqual(
                        pattern["level"], facts.known(int(basic["Required Level"]))
                    )
                    if "Vocations" in basic:
                        self.assertCountEqual(
                            pattern["vocations"]["value"],
                            [
                                VOCATIONS[name]
                                for name in basic["Vocations"].split(", ")
                            ],
                        )
        for identifier in (
            43864,
            43865,
            43866,
            51260,
            51261,
            51262,
            51263,
            51264,
            51269,
        ):
            with self.subTest(identifier=identifier):
                count = int(capture[identifier]["basic"]["Imbuement Slots"])
                self.assertEqual(
                    facts.fact(
                        self.profiles[f"{facts.PREFIX}{identifier}"],
                        "imbuement",
                        "slot_count",
                    ),
                    facts.known(count),
                )

    def test_published_augments_match_cyclopedia_and_catalogue(self):
        capture = load_capture(facts.ROOT)
        manifest = json.loads((facts.ROOT / facts.MANIFEST).read_bytes())
        catalog = json.loads(
            (facts.ROOT / "content" / manifest["catalog"]["path"]).read_bytes()
        )
        names = {row["bundle"]["spell"]["name"].lower() for row in catalog["bundles"]}
        for identifier in (
            50259,
            51266,
            51267,
            52348,
            *range(52350, 52355),
            *range(53229, 53234),
        ):
            with self.subTest(identifier=identifier):
                actual = self.profiles[f"{facts.PREFIX}{identifier}"]["attributes"][
                    "spell_augments"
                ]
                self.assertCountEqual(
                    actual, global_augments(capture[identifier]["basic"]["Augments"])
                )
                self.assertLessEqual({augment["spell"] for augment in actual}, names)

    def test_charged_ethereal_ring_requires_a_complete_timed_pair_to_equip(self):
        charged = self.profiles[facts.CHARGED_RING]
        targets = facts.fact(charged, "use_transform", "targets")
        self.assertIsNotNone(targets)
        [equip] = [target for target in targets if target["kind"] == "EQUIP"]
        self.assertEqual(
            equip["target"],
            facts.known({"key": f"{facts.PREFIX}50148", "revision": "definition-r1"}),
        )
