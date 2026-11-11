import copy
import json
import unittest
import xml.etree.ElementTree as ET
from functools import lru_cache

import grizzly_shop_profiles as owner
import questpc4_treasure_reward_profiles as scoped
import test_questpc4_stag_item_profiles as fixtures


@lru_cache(maxsize=1)
def definitions():
    return {
        row["definition"]["identity"]["key"]: row["definition"]
        for path in (fixtures.ROOT / "content/items/definitions").glob("items-*.json")
        for row in json.loads(path.read_bytes())["records"]
    }


def fixture_for(item_id):
    values = fixtures.StagPhysicalItemTests().fixture()
    values[0] = item_id
    values[-1] = definitions()[f"oteryn:item.tibia.i{item_id}"]
    return values


class TreasureRewardTests(unittest.TestCase):
    def fixture(self, item_id):
        values = fixture_for(item_id)
        if item_id == 9204:
            raw = '<item id="9204"><attribute key="primarytype" value="documents and papers"/><attribute key="readable" value="1"/><attribute key="weight" value="200"/></item>'
        else:
            target = item_id + 1 if item_id < 9619 else 9616
            raw = f'<item id="{item_id}"><attribute key="description" value="It\'s the only left-over from a famous pirate treasure"/><attribute key="rotateto" value="{target}"/><attribute key="weight" value="4500"/></item>'
        values[1] = ET.fromstring(raw)
        return values

    def test_poem_qualifies_readability_without_inventing_text_or_writeability(self):
        row = scoped.qualify(*self.fixture(9204))
        text = row["semantics"]["readable_writeable"]["value"]
        self.assertEqual(text["readable"], owner.source.K(True))
        for name in (
            "writeable",
            "distance_read",
            "max_text_length",
            "write_once_target",
        ):
            self.assertEqual(text[name], owner.source.U)

    def test_four_rotations_keep_exact_target_identity_and_unknown_other_operations(
        self,
    ):
        for item_id in (9616, 9617, 9618, 9619):
            row = scoped.qualify(*self.fixture(item_id))
            targets = row["semantics"]["use_transform"]["value"]["targets"]
            target = item_id + 1 if item_id < 9619 else 9616
            rotation = next(t for t in targets if t["kind"] == "ROTATE")
            self.assertEqual(
                rotation["target"],
                owner.source.K(
                    {"key": f"oteryn:item.tibia.i{target}", "revision": "definition-r1"}
                ),
            )
            self.assertTrue(
                all(
                    t["target"] == owner.source.U
                    for t in targets
                    if t["kind"] != "ROTATE"
                )
            )

    def test_accepted_trade_restrictions_survive_importer_defaults(self):
        for item_id in scoped.IDS:
            values = self.fixture(item_id)
            row = scoped.qualify(*values)
            self.assertEqual(
                row["semantics"]["trade_restrictions"],
                values[-1]["semantics"]["trade_restrictions"],
            )

    def test_changed_read_flag_and_rotation_target_are_rejected(self):
        for item_id, key in ((9204, "readable"), (9616, "rotateto")):
            values = self.fixture(item_id)
            values[1].find(f"attribute[@key='{key}']").set("value", "0")
            with self.assertRaises(ValueError):
                scoped.qualify(*values)

    def test_custom_script_and_special_instance_flags_are_rejected(self):
        values = self.fixture(9616)
        ET.SubElement(values[1], "attribute", key="script", value="unknown")
        with self.assertRaisesRegex(ValueError, "CUSTOM_SCRIPT"):
            scoped.qualify(*values)
        for flag in ("cumulative", "container", "liquidcontainer", "liquidpool"):
            values = self.fixture(9616)
            values[2]["flags"]["flags." + flag] = True
            with self.assertRaisesRegex(ValueError, "INSTANCE_FLAGS"):
                scoped.qualify(*values)

    def test_conflicting_known_canonical_readability_is_rejected(self):
        values = self.fixture(9204)
        values[-1] = copy.deepcopy(values[-1])
        values[-1]["semantics"]["readable_writeable"] = owner.source.K(
            {"readable": owner.source.K(False)}
        )
        with self.assertRaisesRegex(
            ValueError, "CANONICAL_CONFLICT:readable_writeable"
        ):
            scoped.qualify(*values)

    def test_reward_capture_cannot_overwrite_input_evidence(self):
        import build_questpc4_stag_item_profiles as inputs
        import build_questpc4_treasure_reward_profiles as rewards

        self.assertNotEqual(rewards.PROOF, inputs.PROOF)
        self.assertEqual(
            rewards.PROOF,
            "tools/content-schema/native-gameplay/questpc4-treasure-reward-profiles.json",
        )

    def test_check_mode_rejects_a_missing_cohort_without_writing(self):
        import tempfile
        from unittest.mock import patch

        import build_questpc4_treasure_reward_profiles as producer
        import refresh_equipment_profile_pin as cli
        from test_refresh_equipment_profile_pin import EquipmentPinTests, document

        source = EquipmentPinTests.SOURCE.replace(
            '&[("QNPCFX", 1)];',
            '&[("QNPCFX", 1),\n    ("QUESTPC4-STAG-ITEMS", 1),\n];',
        ).replace("            + npc_additions.len()\n", "")
        packet = {
            "schema": producer.SCHEMA,
            "producer": producer.PRODUCER,
            "records": [
                {"production_definition": {"production_key": f"oteryn:item.tibia.i{i}"}}
                for i in scoped.IDS
            ],
        }
        files = {
            "content/items/definitions/spell-native-profiles.json": document(
                0, 1, 2, 3, "stag", *(f"oteryn:item.tibia.i{i}" for i in scoped.IDS)
            ),
            "content/items/definitions/srx1-item-updates.json": document(4),
            "tools/content-schema/native-gameplay/qengine-reward-item-profiles.json": document(
                0
            ),
            "tools/content-schema/native-gameplay/qnpcfx-item-profiles.json": document(
                1
            ),
            "content/items/definitions/starting-loadout-profiles.json": document(),
            producer.PROOF: packet,
        }
        with tempfile.TemporaryDirectory() as tmp:
            root = fixtures.ROOT.__class__(tmp)
            target = root / "pin.rs"
            target.write_text(source)
            for name, value in files.items():
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(json.dumps(value))
            with (
                patch.object(cli, "ROOT", root),
                patch.object(cli, "TEST", target),
                patch("sys.argv", ["refresh_equipment_profile_pin.py", "--check"]),
                self.assertRaisesRegex(SystemExit, "equipment cohort pin is stale"),
            ):
                cli.main()
            self.assertEqual(target.read_text(), source)

    def test_pin_rejects_duplicate_and_foreign_reward_identities(self):
        import build_questpc4_treasure_reward_profiles as producer
        from questpc4_treasure_reward_equipment_pin import refresh

        packet = {
            "schema": producer.SCHEMA,
            "producer": producer.PRODUCER,
            "records": [
                {"production_definition": {"production_key": f"oteryn:item.tibia.i{i}"}}
                for i in scoped.IDS
            ],
        }
        source = '    ("QUESTPC4-STAG-ITEMS", 1),'
        result = refresh(source, packet)
        self.assertEqual(result, source + '\n    ("QUESTPC4-TREASURE-REWARDS", 5),')
        self.assertEqual(refresh(result, packet), result)
        for item_id in (9204, 1):
            bad = copy.deepcopy(packet)
            bad["records"][-1]["production_definition"]["production_key"] = (
                f"oteryn:item.tibia.i{item_id}"
            )
            with self.assertRaisesRegex(ValueError, "SOURCE_COHORT"):
                refresh(source, bad)
