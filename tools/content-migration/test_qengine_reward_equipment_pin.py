import copy
import unittest

from qengine_reward_equipment_pin import IDS, refresh


class RewardIdentityPin(unittest.TestCase):
    SOURCE = '''    assert!(added_keys.insert(key));
    assert_eq!(profile.attributes.speed_bonus, Some(0), "{key}");
    assert_eq!(added_keys.len(), 60);
    let packets = [("qengine-reward-item-profiles.json", 60)];
'''

    def packet(self):
        return {'records': [{'production_definition': {'production_key': f'oteryn:item.tibia.i{i}'}}
                            for i in sorted(IDS)]}

    def test_every_declared_source_identity_is_pinned_without_removing_raw_checks(self):
        result = refresh(self.SOURCE, self.packet())
        self.assertIn(f'assert_eq!(added_keys.len(), {len(IDS)});', result)
        self.assertIn(f'("qengine-reward-item-profiles.json", {len(IDS)})', result)
        for i in IDS:
            self.assertIn(f'"oteryn:item.tibia.i{i}"', result)
        self.assertIn('assert!(added_keys.insert(key));', result)
        self.assertIn('assert_eq!(profile.attributes.speed_bonus, Some(0), "{key}");', result)
        self.assertEqual(refresh(result, self.packet()), result)

    def test_missing_foreign_and_duplicate_records_cannot_regenerate_a_lower_pin(self):
        for mode in ('missing', 'foreign', 'duplicate'):
            packet = copy.deepcopy(self.packet())
            if mode == 'missing':
                packet['records'].pop()
            elif mode == 'foreign':
                packet['records'][0]['production_definition']['production_key'] = 'foreign'
            else:
                packet['records'].append(packet['records'][0])
            with self.assertRaises(ValueError, msg=mode):
                refresh(self.SOURCE, packet)
