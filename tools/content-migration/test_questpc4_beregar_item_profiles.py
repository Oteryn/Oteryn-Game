import copy
import unittest
import xml.etree.ElementTree as ET

import build_questpc4_beregar_item_profiles as profile
import grizzly_shop_profiles as owner
import questpc4_beregar_legs_profile as legs
from questpc4_beregar_equipment_pin import refresh
from test_questpc4_stag_item_profiles import ROOT, definitions


class BeregarCustodyTests(unittest.TestCase):
    def fixture(self, item_id=8774):
        weight = 30000 if item_id == 8774 else 90
        node = ET.fromstring(
            f'<item id="{item_id}" name="fixture"><attribute key="weight" value="{weight}"/></item>'
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

    def test_both_exact_source_items_remain_plain_and_portable(self):
        for item_id in profile.PLAIN_IDS:
            row = profile.qualify(*self.fixture(item_id))
            self.assertEqual(
                row["production_definition"]["production_key"],
                f"oteryn:item.tibia.i{item_id}",
            )
            self.assertEqual(row["semantics"]["equipment"], owner.source.NA)
            self.assertEqual(row["semantics"]["protection"], {"state": "UNKNOWN"})
            self.assertEqual(
                row["admission"]["legal_destinations"], ["CharacterInventory", "Ground"]
            )

    def test_source_weight_or_identity_drift_is_rejected(self):
        values = self.fixture()
        values[1][0].set("value", "1")
        with self.assertRaises(ValueError):
            profile.qualify(*values)
        values = self.fixture()
        values[0] = 8775
        with self.assertRaisesRegex(ValueError, "IDENTITY"):
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

    def test_pin_uses_captured_cohort_and_is_idempotent(self):
        source = '    ("QUESTPC4-YALAHAR-ITEMS", 11),'
        result = refresh(source, self.packet())
        self.assertEqual(result, source + '\n    ("QUESTPC4-BEREGAR-ITEMS", 3),')
        self.assertEqual(refresh(result, self.packet()), result)

    def test_pin_rejects_duplicate_or_wrong_key_and_missing_anchor(self):
        for keys in ((3398, 8774, 8774), (3398, 8774, 1)):
            packet = copy.deepcopy(self.packet())
            for row, key in zip(packet["records"], keys):
                row["production_definition"]["production_key"] = (
                    f"oteryn:item.tibia.i{key}"
                )
            with self.assertRaisesRegex(ValueError, "SOURCE_COHORT"):
                refresh('("QUESTPC4-YALAHAR-ITEMS", 11),', packet)
        with self.assertRaisesRegex(ValueError, "PIN_ANCHOR"):
            refresh("", self.packet())


class BeregarLegsTests(unittest.TestCase):
    def fixture_legs(self):
        values = BeregarCustodyTests().fixture(3398)
        values[1] = ET.fromstring(
            """<item id="3398"><attribute key="primarytype" value="legs"/><attribute key="absorbpercentphysical" value="3"/><attribute key="armor" value="7"/><attribute key="weight" value="4900"/><attribute key="script" value="moveevent"><attribute key="slot" value="legs"/></attribute></item>"""
        )
        return values

    def test_source_armor_resistance_and_slot_survive(self):
        row = legs.qualify(*self.fixture_legs())
        self.assertEqual(
            row["semantics"]["protection"],
            self.fixture_legs()[-1]["semantics"]["protection"],
        )
        self.assertEqual(
            row["semantics"]["equipment"]["value"]["patterns"]["value"][0][
                "primary_slot"
            ],
            owner.source.K("LEGS"),
        )

    def test_changed_source_armor_resistance_or_script_is_rejected(self):
        for index in (1, 2, 3):
            values = self.fixture_legs()
            values[1][index].set("value", "0")
            with self.assertRaisesRegex(ValueError, "SOURCE_ATTRIBUTES"):
                legs.qualify(*values)
        values = self.fixture_legs()
        values[1][-1][0].set("value", "ammo")
        with self.assertRaisesRegex(ValueError, "SOURCE_SCRIPT"):
            legs.qualify(*values)

    def test_conflicting_canonical_armor_slot_and_client_flags_are_rejected(self):
        values = self.fixture_legs()
        values[-1] = copy.deepcopy(values[-1])
        values[-1]["semantics"]["protection"]["value"]["armor"] = owner.source.K(8)
        with self.assertRaisesRegex(ValueError, "CANONICAL_PROTECTION"):
            legs.qualify(*values)
        values = self.fixture_legs()
        values[-1] = copy.deepcopy(values[-1])
        values[-1]["semantics"]["equipment"]["value"]["patterns"]["value"][0][
            "primary_slot"
        ] = owner.source.K("EXTRA")
        with self.assertRaisesRegex(ValueError, "CANONICAL_EQUIPMENT"):
            legs.qualify(*values)
        for flag in ("cumulative", "container", "liquidcontainer", "liquidpool"):
            values = self.fixture_legs()
            values[2]["flags"]["flags." + flag] = True
            with self.assertRaisesRegex(ValueError, "INSTANCE_FLAGS"):
                legs.qualify(*values)

    def test_unbound_or_different_identity_is_rejected(self):
        values = self.fixture_legs()
        values[4] = dict(values[4])
        values[4].pop("3398")
        with self.assertRaisesRegex(ValueError, "EXACT_BINDING"):
            legs.qualify(*values)
        values = self.fixture_legs()
        values[-1] = copy.deepcopy(values[-1])
        values[-1]["identity"]["key"] = "oteryn:item.tibia.i3399"
        with self.assertRaisesRegex(ValueError, "IDENTITY"):
            legs.qualify(*values)
