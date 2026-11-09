"""Source membership, not the generated union, fixes the QENGINE assertion."""
import copy
import json
import unittest

import qengine_reward_equipment_pin as pin
from refresh_equipment_profile_pin import ROOT, TEST


class QengineRewardPinTests(unittest.TestCase):
    def test_source_identity_pin_rejects_missing_or_duplicate_rows(self):
        packet = json.loads((ROOT / 'tools/content-schema/native-gameplay/qengine-reward-item-profiles.json').read_bytes())
        source = TEST.read_text()
        refreshed = pin.refresh(source, packet)
        self.assertIn(f'assert_eq!(added_keys.len(), {len(pin.IDS)});', refreshed)
        self.assertEqual(pin.refresh(refreshed, packet), refreshed)
        for duplicate in (False, True):
            broken = copy.deepcopy(packet)
            broken['records'].pop()
            if duplicate:
                broken['records'].append(broken['records'][0])
            with self.assertRaisesRegex(ValueError, 'QENGINE_REWARD_SOURCE_COHORT'):
                pin.refresh(source, broken)


if __name__ == '__main__':
    unittest.main()
