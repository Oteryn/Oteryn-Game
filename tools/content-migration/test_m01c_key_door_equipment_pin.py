"""M01c's two source identities independently declare the generated pin extent."""
import copy
import json
import unittest

import key_door_item_profiles as keys
import m01c_key_door_equipment_pin as pin
from refresh_equipment_profile_pin import ROOT, TEST


class M01cKeyDoorPinTests(unittest.TestCase):
    def test_refresh_is_source_bounded_and_idempotent(self):
        packet = json.loads((ROOT / keys.PATH).read_bytes())
        source = TEST.read_text()
        refreshed = pin.refresh(source, packet)
        self.assertIn('("M01C-KEY-DOORS", 2)', refreshed)
        self.assertEqual(pin.refresh(refreshed, packet), refreshed)
        broken = copy.deepcopy(packet)
        broken['records'].pop()
        with self.assertRaisesRegex(ValueError, 'M01C_KEY_DOOR_COHORT'):
            pin.refresh(source, broken)


if __name__ == '__main__':
    unittest.main()
