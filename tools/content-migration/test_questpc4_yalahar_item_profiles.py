import copy
import unittest
import xml.etree.ElementTree as ET

import build_questpc4_yalahar_item_profiles as profile
import grizzly_shop_profiles as owner
from questpc4_yalahar_equipment_pin import refresh
from test_questpc4_stag_item_profiles import ROOT, definitions


class YalaharCustodyTests(unittest.TestCase):
    def fixture(self, item_id=8758):
        weights = dict(
            zip(profile.IDS, (280, 500, 80, 750, 500, 1100, 200, 180, 4600, 900, 900))
        )
        node = ET.fromstring(
            f'<item id="{item_id}" name="fixture"><attribute key="weight" value="{weights[item_id]}"/></item>'
        )
        physical = {
            "movable": True,
            "pickupable": True,
            "block_solid": False,
            "block_projectile": False,
            "has_height": False,
        }
        bindings = owner.source.binding_index(
            (ROOT / "imports/crystalserver/bindings/items.json").read_bytes()
        )
        return [
            item_id,
            node,
            {"flags": {}},
            physical,
            bindings,
            definitions()[f"oteryn:item.tibia.i{item_id}"],
        ]

    def test_exact_plain_cohort_retains_unknown_protection_and_no_equipment(self):
        for item_id in profile.IDS:
            with self.subTest(item_id=item_id):
                row = profile.qualify(*self.fixture(item_id))
                self.assertEqual(
                    row["production_definition"]["production_key"],
                    f"oteryn:item.tibia.i{item_id}",
                )
                self.assertEqual(row["semantics"]["protection"], {"state": "UNKNOWN"})
                self.assertEqual(row["semantics"]["equipment"], owner.source.NA)
                self.assertEqual(
                    row["admission"]["legal_destinations"],
                    ["CharacterInventory", "Ground"],
                )

    def test_functional_or_instance_flags_are_rejected(self):
        for flag in ("cumulative", "container", "liquidcontainer", "liquidpool"):
            values = self.fixture()
            values[2]["flags"]["flags." + flag] = True
            with self.assertRaisesRegex(ValueError, "INSTANCE_FLAGS"):
                profile.qualify(*values)

    def test_only_selected_member_of_source_range_is_admitted(self):
        values = self.fixture(8822)
        values[1].attrib.pop("id")
        values[1].set("fromid", "8822")
        values[1].set("toid", "8825")
        self.assertEqual(
            profile.qualify(*values)["production_definition"]["production_key"],
            "oteryn:item.tibia.i8822",
        )
        values[0] = 8823
        with self.assertRaisesRegex(ValueError, "IDENTITY"):
            profile.qualify(*values)

    def test_custom_attributes_and_scripts_are_rejected(self):
        for raw in (
            '<attribute key="armor" value="0"/>',
            '<attribute key="duration" value="30"/>',
            '<attribute key="script" value="action"/>',
        ):
            values = self.fixture()
            values[1].append(ET.fromstring(raw))
            with self.assertRaisesRegex(ValueError, "SOURCE_ATTRIBUTES"):
                profile.qualify(*values)

    def test_identity_binding_and_weight_conflicts_are_rejected(self):
        for mutation in ("identity", "binding", "weight"):
            values = self.fixture()
            if mutation == "identity":
                values[1].set("id", "1")
            elif mutation == "binding":
                values[4] = copy.deepcopy(values[4])
                values[4]["8758"]["disposition"] = "UNRESOLVED"
            else:
                values[1][0].set("value", "9999")
            with self.assertRaises(ValueError):
                profile.qualify(*values)

    def packet(self):
        return {
            "schema": profile.SCHEMA,
            "producer": profile.PRODUCER,
            "records": [
                {"production_definition": {"production_key": f"oteryn:item.tibia.i{i}"}}
                for i in profile.IDS
            ],
        }

    def test_pin_insertion_is_source_bounded_and_idempotent(self):
        source = '    ("QUESTPC4-STAG-ITEMS", 6),'
        generated = refresh(source, self.packet())
        self.assertEqual(generated, source + '\n    ("QUESTPC4-YALAHAR-ITEMS", 11),')
        self.assertEqual(refresh(generated, self.packet()), generated)

    def test_pin_rejects_missing_duplicate_or_unrelated_source_keys(self):
        original = self.packet()
        for rows in (
            original["records"][:-1],
            [original["records"][0]] * len(profile.IDS),
            [{"production_definition": {"production_key": "oteryn:item.tibia.i1"}}]
            + original["records"][1:],
        ):
            packet = copy.deepcopy(original)
            packet["records"] = rows
            with self.assertRaisesRegex(ValueError, "SOURCE_COHORT"):
                refresh('("QUESTPC4-STAG-ITEMS", 6),', packet)

    def test_pin_rejects_ambiguous_or_missing_anchor(self):
        for source in (
            "",
            '("QUESTPC4-STAG-ITEMS", 6),\n("QUESTPC4-STAG-ITEMS", 6),',
            '("QUESTPC4-YALAHAR-ITEMS", 11),' * 2,
        ):
            with self.assertRaises(ValueError):
                refresh(source, self.packet())
