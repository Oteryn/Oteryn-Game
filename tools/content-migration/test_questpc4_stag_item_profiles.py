"""Source armour-zero/ammo qualification cannot erase non-neutral or custom rules."""

import copy
import functools
import json
import unittest
import xml.etree.ElementTree as ET
from pathlib import Path

import grizzly_shop_profiles as source_owner
import questpc4_stag_item_profiles as qualifier

ROOT = Path(__file__).resolve().parents[2]


@functools.cache
def definitions():
    return {
        row["definition"]["identity"]["key"]: row["definition"]
        for path in (ROOT / "content/items/definitions").glob("items-*.json")
        for row in json.loads(path.read_bytes())["records"]
    }


class StagPhysicalItemTests(unittest.TestCase):
    def fixture(self, item_id=52824):
        definition = definitions()[f"oteryn:item.tibia.i{item_id}"]
        bindings = source_owner.source.binding_index(
            (ROOT / "imports/crystalserver/bindings/items.json").read_bytes()
        )
        material = item_id in (52586, 52635, 52743, 52758)
        weight = 50 if item_id == 52586 else 2000 if material else 3000
        equipment = (
            ""
            if material
            else '<attribute key="armor" value="0"/><attribute key="script" value="moveevent"><attribute key="slot" value="ammo"/></attribute>'
        )
        node = ET.fromstring(
            f'<item id="{item_id}" name="fixture"><attribute key="description" value="fixture"/>'
            f'<attribute key="weight" value="{weight}"/>{equipment}</item>'
        )
        physical = {
            "movable": True,
            "pickupable": True,
            "block_solid": False,
            "block_projectile": False,
            "has_height": False,
        }
        flags = {"flags.cumulative": True} if material and item_id != 52586 else {}
        return [item_id, node, {"flags": flags}, physical, bindings, definition]

    def test_plain_source_items_keep_weight_stack_and_unknown_protection(self):
        for item_id in (52586, 52635, 52743, 52758):
            with self.subTest(item_id=item_id):
                record = qualifier.qualify(*self.fixture(item_id))
                semantics = record["semantics"]
                self.assertEqual(semantics["protection"], {"state": "UNKNOWN"})
                self.assertEqual(semantics["equipment"], {"state": "NOT_APPLICABLE"})
                self.assertEqual(
                    semantics["physical"]["value"]["weight"],
                    {"state": "KNOWN", "value": 50 if item_id == 52586 else 2000},
                )
                self.assertEqual(
                    semantics["stack"]["value"]["stack_max"],
                    {"state": "NOT_APPLICABLE"}
                    if item_id == 52586
                    else {"state": "KNOWN", "value": 100},
                )

    def test_plain_item_cannot_borrow_equipment_or_custom_script(self):
        for attribute in (
            '<attribute key="armor" value="0"/>',
            '<attribute key="script" value="moveevent"><attribute key="slot" value="ammo"/></attribute>',
        ):
            values = self.fixture(52635)
            values[1].append(ET.fromstring(attribute))
            with self.assertRaisesRegex(ValueError, "MATERIAL_SOURCE_ATTRIBUTES"):
                qualifier.qualify(*values)

    def test_plain_item_weight_and_stack_conflicts_remain_rejected(self):
        values = self.fixture(52635)
        values[1].find("attribute[@key='weight']").set("value", "9001")
        with self.assertRaises(ValueError):
            qualifier.qualify(*values)
        values = self.fixture(52635)
        values[2]["flags"].clear()
        with self.assertRaises(ValueError):
            qualifier.qualify(*values)

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

    def test_compass_keeps_exact_identity_and_neutral_equipment(self):
        record = qualifier.qualify(*self.fixture(52759))
        self.assertEqual(
            record["production_definition"]["production_key"],
            "oteryn:item.tibia.i52759",
        )
        self.assertEqual(
            record["semantics"]["protection"]["value"]["armor"],
            {"state": "KNOWN", "value": 0},
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
