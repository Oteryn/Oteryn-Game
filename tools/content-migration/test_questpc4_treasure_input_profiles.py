import copy
import unittest
import xml.etree.ElementTree as ET

import grizzly_shop_profiles as owner
import questpc4_treasure_input_profiles as scoped
import test_questpc4_beregar_item_profiles as fixture_owner


class TreasureInputTests(unittest.TestCase):
    def fixture(self, item_id=9181):
        values = fixture_owner.BeregarCustodyTests().fixture(item_id)
        weight = (
            790
            if item_id in (9181, 9182, 9183, 9184)
            else 650
            if item_id == 9177
            else 200
        )
        node = ET.fromstring(
            f'<item id="{item_id}"><attribute key="weight" value="{weight}"/></item>'
        )
        if item_id == 9177:
            ET.SubElement(
                node,
                "attribute",
                key="description",
                value="You read the numbers 3613 on the back of the doll",
            )
        values[1] = node
        return values

    def test_all_six_inputs_preserve_plain_custody_without_equipment(self):
        for item_id in scoped.IDS:
            row = scoped.qualify(*self.fixture(item_id))
            self.assertEqual(
                row["admission"]["legal_destinations"], ["CharacterInventory", "Ground"]
            )
            self.assertEqual(row["semantics"]["equipment"], owner.source.NA)
            self.assertEqual(row["semantics"]["protection"], owner.source.U)

    def test_other_map_and_doll_identities_cannot_enter_the_cohort(self):
        for item_id in (9178, 9179, 9205):
            with self.assertRaisesRegex(ValueError, "PLAIN_ITEM_IDENTITY"):
                scoped.qualify(*self.fixture(item_id))

    def test_selected_code_cannot_be_substituted(self):
        values = self.fixture(9177)
        values[1].find("attribute[@key='description']").set(
            "value", "You read the numbers 2543 on the back of the doll"
        )
        with self.assertRaisesRegex(ValueError, "TREASURE_INPUT_SELECTED_CODE"):
            scoped.qualify(*values)

    def test_reading_duration_and_custom_scripts_remain_outside_plain_custody(self):
        for key in ("readable", "duration", "script"):
            values = self.fixture(9186)
            ET.SubElement(values[1], "attribute", key=key, value="1")
            with self.assertRaisesRegex(ValueError, "PLAIN_ITEM_SOURCE_ATTRIBUTES"):
                scoped.qualify(*values)

    def test_special_instance_flags_are_rejected(self):
        for flag in ("cumulative", "container", "liquidcontainer", "liquidpool"):
            values = self.fixture()
            values[2]["flags"]["flags." + flag] = True
            with self.assertRaisesRegex(ValueError, "PLAIN_ITEM_INSTANCE_FLAGS"):
                scoped.qualify(*values)

    def test_conflicting_known_canonical_mass_is_rejected(self):
        values = self.fixture()
        values[-1] = copy.deepcopy(values[-1])
        values[-1]["semantics"]["physical"]["value"]["weight"] = owner.source.K(1)
        with self.assertRaises(ValueError):
            scoped.qualify(*values)

    def test_pin_requires_the_closed_six_item_capture(self):
        import build_questpc4_treasure_input_profiles as producer
        from questpc4_treasure_equipment_pin import refresh

        packet = {
            "schema": producer.SCHEMA,
            "producer": producer.PRODUCER,
            "records": [
                {"production_definition": {"production_key": f"oteryn:item.tibia.i{i}"}}
                for i in scoped.IDS
            ],
        }
        source = '    ("QUESTPC4-SPECIAL-ITEMS", 5),'
        result = refresh(source, packet)
        self.assertEqual(result, source + '\n    ("QUESTPC4-TREASURE-INPUTS", 6),')
        self.assertEqual(refresh(result, packet), result)
        for item_id in (9181, 1):
            bad = copy.deepcopy(packet)
            bad["records"][-1]["production_definition"]["production_key"] = (
                f"oteryn:item.tibia.i{item_id}"
            )
            with self.assertRaisesRegex(ValueError, "SOURCE_COHORT"):
                refresh(source, bad)
