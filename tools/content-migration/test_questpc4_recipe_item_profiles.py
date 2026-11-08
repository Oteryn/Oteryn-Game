import copy
import unittest
import xml.etree.ElementTree as ET

import build_questpc4_recipe_item_profiles as profile
import grizzly_shop_profiles as owner
from questpc4_recipe_equipment_pin import refresh
from test_questpc4_stag_item_profiles import ROOT, definitions


class RecipeCustodyTests(unittest.TestCase):
    def fixture(self, item_id=14682):
        weight = dict(
            zip(
                profile.IDS,
                (100, 3050, 180, 60, 200, 1100, 1750, 280, 50, 850, 850, 200, 830),
            )
        )[item_id]
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

    def test_thirteen_exact_source_items_remain_plain_and_portable(self):
        for item_id in profile.IDS:
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
        values[0] = 14683
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
        source = '    ("QUESTPC4-BEREGAR-ITEMS", 3),'
        result = refresh(source, self.packet())
        self.assertEqual(result, source + '\n    ("QUESTPC4-RECIPE-ITEMS", 13),')
        self.assertEqual(refresh(result, self.packet()), result)

    def test_pin_rejects_duplicate_or_wrong_key_and_missing_anchor(self):
        for replacement in (profile.IDS[-2], 1):
            packet = copy.deepcopy(self.packet())
            packet["records"][-1]["production_definition"]["production_key"] = (
                f"oteryn:item.tibia.i{replacement}"
            )
            with self.assertRaisesRegex(ValueError, "SOURCE_COHORT"):
                refresh('("QUESTPC4-BEREGAR-ITEMS", 3),', packet)
        with self.assertRaisesRegex(ValueError, "PIN_ANCHOR"):
            refresh("", self.packet())
