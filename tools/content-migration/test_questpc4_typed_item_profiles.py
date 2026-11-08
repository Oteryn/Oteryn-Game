import copy
import unittest
import xml.etree.ElementTree as ET

import grizzly_shop_profiles as owner
import questpc4_typed_item_profiles as typed
import test_questpc4_beregar_item_profiles as fixture_owner


class TypedCustodyTests(unittest.TestCase):
    def fixture(self, item_id=5949):
        values = fixture_owner.BeregarCustodyTests().fixture(item_id)
        if item_id == 5949:
            values[1] = ET.fromstring(
                '<item id="5949"><attribute key="primarytype" value="containers"/><attribute key="containersize" value="20"/><attribute key="weight" value="1800"/><attribute key="imbuementslot" value="1"/><attribute key="script" value="moveevent"><attribute key="slot" value="backpack"/></attribute></item>'
            )
            values[2] = {"flags": {"flags.container": True, "flags.clothes": True}}
        else:
            weight = {24382: 1100, 21506: 180, 7785: 1800, 52632: 1000}[item_id]
            values[1] = ET.fromstring(
                f'<item id="{item_id}"><attribute key="weight" value="{weight}"/></item>'
            )
            values[2] = {"flags": {"flags.cumulative": True}}
        return values

    def test_material_stack_capacity_is_source_owned(self):
        for item_id in typed.STACK_IDS:
            row = typed.qualify(*self.fixture(item_id))
            self.assertEqual(
                row["semantics"]["stack"]["value"]["stack_max"], owner.source.K(100)
            )
            self.assertEqual(row["semantics"]["equipment"], owner.source.NA)
            self.assertEqual(row["semantics"]["protection"], owner.source.U)
            self.assertEqual(row["semantics"]["charges"], owner.source.NA)
            self.assertEqual(row["semantics"]["temporal"], owner.source.NA)

    def test_backpack_preserves_capacity_and_container_slot(self):
        row = typed.qualify(*self.fixture())
        self.assertEqual(
            row["semantics"]["container"],
            owner.source.K({"capacity": owner.source.K(20)}),
        )
        pattern = row["semantics"]["equipment"]["value"]["patterns"]["value"][0]
        self.assertEqual(pattern["primary_slot"], owner.source.K("CONTAINER"))
        self.assertEqual(
            row["admission"]["legal_destinations"], ["CharacterInventory", "Ground"]
        )

    def test_wrong_stack_or_fluid_flags_are_rejected(self):
        for item_id in typed.STACK_IDS:
            values = self.fixture(item_id)
            values[2]["flags"]["flags.cumulative"] = False
            with self.assertRaisesRegex(ValueError, "STACK_FLAGS"):
                typed.qualify(*values)
            values = self.fixture(item_id)
            values[2]["flags"]["flags.liquidcontainer"] = True
            with self.assertRaisesRegex(ValueError, "FLUID"):
                typed.qualify(*values)

    def test_custom_stack_script_is_not_enabled(self):
        values = self.fixture(7785)
        ET.SubElement(values[1], "attribute", key="script", value="action")
        with self.assertRaisesRegex(ValueError, "SOURCE_ATTRIBUTES"):
            typed.qualify(*values)

    def test_changed_backpack_capacity_or_slot_is_rejected(self):
        values = self.fixture()
        values[1][1].set("value", "21")
        with self.assertRaisesRegex(ValueError, "SOURCE_ATTRIBUTES"):
            typed.qualify(*values)
        values = self.fixture()
        values[1][-1][0].set("value", "ammo")
        with self.assertRaisesRegex(ValueError, "SOURCE_SCRIPT"):
            typed.qualify(*values)

    def test_conflicting_canonical_container_fact_is_rejected(self):
        values = self.fixture()
        values[-1] = copy.deepcopy(values[-1])
        values[-1]["semantics"]["container"]["value"]["capacity"] = owner.source.K(21)
        with self.assertRaisesRegex(ValueError, "CANONICAL_CONFLICT:container"):
            typed.qualify(*values)

    def test_pin_uses_only_the_exact_typed_source_cohort(self):
        import build_questpc4_typed_item_profiles as producer
        from questpc4_typed_equipment_pin import refresh

        packet = {
            "schema": producer.SCHEMA,
            "producer": producer.PRODUCER,
            "records": [
                {"production_definition": {"production_key": f"oteryn:item.tibia.i{i}"}}
                for i in typed.IDS
            ],
        }
        source = '    ("QUESTPC4-RECIPE-ITEMS", 13),'
        result = refresh(source, packet)
        self.assertEqual(result, source + '\n    ("QUESTPC4-TYPED-ITEMS", 5),')
        self.assertEqual(refresh(result, packet), result)
        for replacement in (typed.IDS[-2], 1):
            bad = copy.deepcopy(packet)
            bad["records"][-1]["production_definition"]["production_key"] = (
                f"oteryn:item.tibia.i{replacement}"
            )
            with self.assertRaisesRegex(ValueError, "SOURCE_COHORT"):
                refresh(source, bad)
