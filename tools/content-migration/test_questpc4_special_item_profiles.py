import copy
import unittest
import xml.etree.ElementTree as ET

import grizzly_shop_profiles as owner
import questpc4_special_item_profiles as special
import test_questpc4_beregar_item_profiles as fixture_owner

XML = {
    8862: '<item id="8862"><attribute key="primarytype" value="armors"/><attribute key="absorbpercentdeath" value="3"/><attribute key="armor" value="16"/><attribute key="weight" value="7000"/><attribute key="script" value="moveevent"><attribute key="level" value="80"/><attribute key="slot" value="armor"/><attribute key="vocation" value="Knight;true, Elite Knight"/></attribute></item>',
    21389: '<item id="21389"><attribute key="description" value="fixture"/><attribute key="duration" value="3600"/><attribute key="weight" value="100"/></item>',
    5552: '<item id="5552"><attribute key="primarytype" value="fluid containers"/><attribute key="allowpickupable" value="1"/><attribute key="weight" value="280"/></item>',
    9171: '<item id="9171"><attribute key="primarytype" value="documents and papers"/><attribute key="description" value="fixture"/><attribute key="maxtextlen" value="1023"/><attribute key="weight" value="200"/></item>',
    21572: '<item id="21572"><attribute key="primarytype" value="rocks"/><attribute key="description" value="fixture"/></item>',
}


class SpecialCustodyTests(unittest.TestCase):
    def fixture(self, item_id):
        values = fixture_owner.BeregarCustodyTests().fixture(item_id)
        values[1] = ET.fromstring(XML[item_id])
        values[2] = {
            "flags": {"flags.clothes": True}
            if item_id == 8862
            else {"flags.liquidcontainer": True}
            if item_id == 5552
            else {}
        }
        if item_id == 21572:
            values[3]["movable"] = False
            values[3]["pickupable"] = False
        return values

    def test_armor_preserves_level_vocation_slot_and_protection(self):
        row = special.qualify(*self.fixture(8862))
        self.assertEqual(
            row["semantics"]["protection"],
            self.fixture(8862)[-1]["semantics"]["protection"],
        )
        pattern = row["semantics"]["equipment"]["value"]["patterns"]["value"][0]
        self.assertEqual(pattern["primary_slot"], owner.source.K("TORSO"))
        self.assertEqual(pattern["level"], owner.source.K(80))
        self.assertEqual(pattern["vocations"], owner.source.K(["KNIGHT"]))

    def test_flower_preserves_absolute_hour_without_inventing_target(self):
        row = special.qualify(*self.fixture(21389))
        self.assertEqual(
            row["semantics"]["temporal"],
            owner.source.K(
                {
                    "consumption_mode": owner.source.K("DURABLE_ABSOLUTE_DEADLINE"),
                    "duration": owner.source.K(3600000),
                    "stop_duration": owner.source.K(False),
                    "decay_target": owner.source.NA,
                }
            ),
        )

    def test_fluid_definition_does_not_invent_rum_subtype(self):
        row = special.qualify(*self.fixture(5552))
        self.assertEqual(
            row["semantics"]["fluid"], owner.source.K({"fluid_type": owner.source.NA})
        )
        self.assertEqual(row["semantics"]["equipment"], owner.source.NA)

    def test_text_length_does_not_authorize_read_or_write(self):
        row = special.qualify(*self.fixture(9171))
        fields = row["semantics"]["readable_writeable"]["value"]
        self.assertEqual(fields["max_text_length"], owner.source.K(1023))
        for field in ("readable", "writeable", "distance_read", "write_once_target"):
            self.assertEqual(fields[field], owner.source.U)

    def test_omitted_mass_uses_existing_importer_loader_default(self):
        row = special.qualify(*self.fixture(21572))
        self.assertEqual(
            row["semantics"]["physical"]["value"]["weight"], owner.source.K(0)
        )
        self.assertEqual(row["semantics"]["fluid"], owner.source.U)
        values = self.fixture(21572)
        ET.SubElement(values[1], "attribute", key="weight", value="1")
        with self.assertRaisesRegex(ValueError, "SOURCE_ATTRIBUTES"):
            special.qualify(*values)

    def test_crystal_rejects_invented_inventory_mobility(self):
        row = special.qualify(*self.fixture(21572))
        self.assertEqual(row["admission"]["legal_destinations"], ["Ground"])
        values = self.fixture(21572)
        values[3]["movable"] = True
        values[3]["pickupable"] = True
        with self.assertRaisesRegex(ValueError, "GROUND_ONLY"):
            special.qualify(*values)

    def test_changed_duration_text_limit_and_armor_are_rejected(self):
        for item_id, key in (
            (21389, "duration"),
            (9171, "maxtextlen"),
            (8862, "armor"),
        ):
            values = self.fixture(item_id)
            values[1].find(f"attribute[@key='{key}']").set("value", "1")
            with self.assertRaisesRegex(ValueError, "SOURCE_ATTRIBUTES"):
                special.qualify(*values)

    def test_fluid_flag_and_armor_script_drift_are_rejected(self):
        values = self.fixture(5552)
        values[2]["flags"]["flags.liquidcontainer"] = False
        with self.assertRaisesRegex(ValueError, "INSTANCE_FLAGS"):
            special.qualify(*values)
        values = self.fixture(8862)
        values[1][-1][0].set("value", "0")
        with self.assertRaisesRegex(ValueError, "ARMOR_SCRIPT"):
            special.qualify(*values)

    def test_conflicting_canonical_armor_is_rejected(self):
        values = self.fixture(8862)
        values[-1] = copy.deepcopy(values[-1])
        values[-1]["semantics"]["protection"]["value"]["armor"] = owner.source.K(0)
        with self.assertRaisesRegex(ValueError, "ARMOR_PROTECTION"):
            special.qualify(*values)

    def test_pin_requires_closed_five_item_cohort(self):
        import build_questpc4_special_item_profiles as producer
        from questpc4_special_equipment_pin import refresh

        packet = {
            "schema": producer.SCHEMA,
            "producer": producer.PRODUCER,
            "records": [
                {"production_definition": {"production_key": f"oteryn:item.tibia.i{i}"}}
                for i in special.IDS
            ],
        }
        source = '    ("QUESTPC4-TYPED-ITEMS", 5),'
        result = refresh(source, packet)
        self.assertEqual(result, source + '\n    ("QUESTPC4-SPECIAL-ITEMS", 5),')
        self.assertEqual(refresh(result, packet), result)
        for replacement in (special.IDS[-2], 1):
            bad = copy.deepcopy(packet)
            bad["records"][-1]["production_definition"]["production_key"] = (
                f"oteryn:item.tibia.i{replacement}"
            )
            with self.assertRaisesRegex(ValueError, "SOURCE_COHORT"):
                refresh(source, bad)
