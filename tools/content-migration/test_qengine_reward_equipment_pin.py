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
import copy
import json
import unittest
from pathlib import Path

from qengine_reward_equipment_pin import refresh


class SourceCohortTests(unittest.TestCase):
    def setUp(self):
        self.packet = json.loads((Path(__file__).resolve().parents[1] /
            "content-schema/native-gameplay/qengine-reward-item-profiles.json").read_bytes())
        self.source = 'assert_eq!(added_keys.len(), 60);\n("qengine-reward-item-profiles.json", 60)'

    def test_regenerates_stale_cardinality_from_the_producer_cohort(self):
        self.assertEqual('assert_eq!(added_keys.len(), 63);\n("qengine-reward-item-profiles.json", 63)', refresh(self.source, self.packet))

    def test_refuses_a_missing_source_identity(self):
        self.packet["records"].pop()
        with self.assertRaisesRegex(ValueError, "SOURCE_COHORT"):
            refresh(self.source, self.packet)

    def test_refuses_an_extra_or_duplicated_identity(self):
        self.packet["records"].append(copy.deepcopy(self.packet["records"][0]))
        with self.assertRaisesRegex(ValueError, "SOURCE_COHORT"):
            refresh(self.source, self.packet)

    def test_refuses_a_changed_source_pin(self):
        self.packet["sources"][0]["sha256"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "SOURCE_COHORT"):
            refresh(self.source, self.packet)


if __name__ == "__main__":
    unittest.main()
