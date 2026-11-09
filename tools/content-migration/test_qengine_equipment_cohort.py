"""A stale count may be regenerated; a changed or duplicated proof identity cannot."""
import unittest
import qengine_equipment_cohort as tool


import json
from pathlib import Path



ROOT = Path(__file__).resolve().parents[2]
TEST = ROOT / 'crates/gameplay/dispatch/src/gameplay_transport/equipment_effects_owner_tests/ability_tests.rs'
PACKET = ROOT / 'tools/content-schema/native-gameplay/qengine-reward-item-profiles.json'


class QengineEquipmentCohortTests(unittest.TestCase):
    def packet(self):
        return {'records': [{'production_definition': {'production_key': f'oteryn:item.tibia.i{i}'}}
                            for i in sorted(tool.IDS)]}

    def test_both_strict_pins_follow_declared_source_ids_and_keep_row_checks(self):
        source = '''// QENGINE2 captures 60 Items.
assert_eq!(added_keys.len(), 60);
("qengine-reward-item-profiles.json", 60)
assert_eq!(actual.attributes.speed_bonus, Some(0));'''
        generated = tool.refresh(source, self.packet())
        self.assertIn(f'assert_eq!(added_keys.len(), {len(tool.IDS)});', generated)
        self.assertIn(f'("qengine-reward-item-profiles.json", {len(tool.IDS)})', generated)
        self.assertIn('assert_eq!(actual.attributes.speed_bonus, Some(0));', generated)
        self.assertEqual(tool.refresh(generated, self.packet()), generated)

    def test_same_count_substitution_and_extra_or_duplicate_rows_refuse(self):
        for kind in ['substitute', 'extra', 'duplicate']:
            packet = self.packet()
            if kind == 'substitute':
                packet['records'][0]['production_definition']['production_key'] = 'oteryn:item.tibia.i999999'
            elif kind == 'extra':
                packet['records'].append({'production_definition': {'production_key': 'oteryn:item.tibia.i999999'}})
            else:
                packet['records'].append(packet['records'][0])
            with self.assertRaisesRegex(ValueError, 'SOURCE_SET'):
                tool.refresh('', packet)


class EquipmentCohortAnchors(unittest.TestCase):
    def setUp(self):
        self.packet = json.loads(PACKET.read_bytes())

    def test_current_recorded_cohort_keeps_both_functional_pins(self):
        result = tool.refresh(TEST.read_text(), self.packet)
        self.assertIn(f'assert_eq!(added_keys.len(), {len(tool.IDS)});', result)
        self.assertIn(f'("qengine-reward-item-profiles.json", {len(tool.IDS)})', result)
        self.assertEqual(tool.refresh(result, self.packet), result)

    def test_unknown_comment_still_fails_closed(self):
        source = ('assert_eq!(added_keys.len(), 1);\n'
                  '("qengine-reward-item-profiles.json", 1)\n'
                  '// unknown cohort\n')
        with self.assertRaisesRegex(ValueError, 'QENGINE_EQUIPMENT_PIN_ANCHOR'):
            tool.refresh(source, self.packet)

    def test_duplicate_functional_pin_still_fails_closed(self):
        source = TEST.read_text() + '\nassert_eq!(added_keys.len(), 1);\n'
        with self.assertRaisesRegex(ValueError, 'QENGINE_EQUIPMENT_PIN_ANCHOR'):
            tool.refresh(source, self.packet)


if __name__ == '__main__':
    unittest.main()
