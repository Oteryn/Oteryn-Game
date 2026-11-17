"""Source armour-zero/ammo qualification cannot erase non-neutral or custom rules."""

import copy
import json
import unittest
import xml.etree.ElementTree as ET
from pathlib import Path

import grizzly_shop_profiles as source_owner
import questpc4_stag_item_profiles as qualifier

ROOT = Path(__file__).resolve().parents[2]


class StagPhysicalItemTests(unittest.TestCase):
    def fixture(self):
        document = json.loads(
            (ROOT / "content/items/definitions/items-28500-28999.json").read_bytes()
        )
        definition = next(
            r["definition"]
            for r in document["records"]
            if r["definition"]["identity"]["key"] == "oteryn:item.tibia.i52824"
        )
        bindings = source_owner.source.binding_index(
            (ROOT / "imports/crystalserver/bindings/items.json").read_bytes()
        )
        node = ET.fromstring(
            '<item id="52824" name="fixture"><attribute key="description" value="fixture"/>'
            '<attribute key="armor" value="0"/><attribute key="weight" value="3000"/>'
            '<attribute key="script" value="moveevent"><attribute key="slot" value="ammo"/></attribute></item>'
        )
        physical = {
            "movable": True,
            "pickupable": True,
            "block_solid": False,
            "block_projectile": False,
            "has_height": False,
        }
        return [52824, node, {"flags": {}}, physical, bindings, definition]

    def test_exact_neutral_source_keeps_armor_and_ammo_semantics(self):
        record = qualifier.qualify(*self.fixture())
        self.assertEqual(
            record["production_definition"]["production_key"],
            "oteryn:item.tibia.i52824",
        )
        self.assertEqual(
            record["semantics"]["protection"]["value"]["armor"],
            {"state": "KNOWN", "value": 0},
        )
        self.assertEqual(
            record["semantics"]["equipment"]["value"]["patterns"]["value"][0][
                "primary_slot"
            ],
            {"state": "KNOWN", "value": "EXTRA"},
        )
        self.assertEqual(
            record["admission"]["legal_destinations"], ["CharacterInventory", "Ground"]
        )

    def test_nonzero_armor_is_not_dropped(self):
        values = self.fixture()
        values[1].find("attribute[@key='armor']").set("value", "1")
        with self.assertRaises(ValueError):
            qualifier.qualify(*values)

    def test_custom_script_and_wrong_slot_remain_closed(self):
        for attribute, value in [("value", "action"), ("slot", "head")]:
            values = self.fixture()
            script = values[1].find("attribute[@key='script']")
            (script if attribute == "value" else script[0]).set("value", value)
            with self.assertRaises(ValueError):
                qualifier.qualify(*values)

    def test_unknown_attribute_remains_closed(self):
        values = self.fixture()
        ET.SubElement(values[1], "attribute", {"key": "duration", "value": "600"})
        with self.assertRaises(ValueError):
            qualifier.qualify(*values)

    def test_conflicting_canonical_armor_is_rejected(self):
        values = self.fixture()
        values[5] = copy.deepcopy(values[5])
        values[5]["semantics"]["protection"]["value"]["armor"]["value"] = 1
        with self.assertRaises(ValueError):
            qualifier.qualify(*values)

    def test_nonexact_binding_is_rejected(self):
        values = self.fixture()
        values[4] = copy.deepcopy(values[4])
        values[4]["52824"]["disposition"] = "INFERRED"
        with self.assertRaisesRegex(ValueError, "EXACT_BINDING"):
            qualifier.qualify(*values)

    def test_world_only_item_cannot_become_an_inventory_reward(self):
        values = self.fixture()
        values[3]["movable"] = False
        with self.assertRaisesRegex(ValueError, "INSTANCE_OWNER"):
            qualifier.qualify(*values)

    def test_other_identity_cannot_borrow_this_qualification(self):
        values = self.fixture()
        values[0] = 52825
        with self.assertRaises(ValueError):
            qualifier.qualify(*values)


if __name__ == "__main__":
    unittest.main()
