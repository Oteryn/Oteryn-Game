"""The real decoder compares the complete frozen Item body after all publishers."""
import hashlib
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


class BossRewardBindingPolicyTests(unittest.TestCase):
    def test_actual_native_extra_binding_rows_match_complete_frozen_policy(self):
        capture = json.loads((ROOT/'tools/content-schema/native-gameplay/boss-reward-yield-profiles.json').read_bytes())
        native = json.loads((ROOT/'content/items/definitions/spell-native-profiles.json').read_bytes())
        actual = {row['production_binding']['external_id']: row for row in native['records']}
        expected = {row['production_binding']['external_id']: row for row in capture['records']}
        for identifier in ('43860', '43946', '43947', '44048'):
            self.assertTrue(identifier in actual, f'actual native Item{identifier} is missing')
            self.assertEqual(digest(actual[identifier]), digest(expected[identifier]),
                             f'complete native Item{identifier} policy differs from frozen source body')
        self.assertEqual(expected['43946']['authoring']['taxonomy'], {'primary': 'Valuables'})
        self.assertEqual(expected['43947']['authoring']['taxonomy'], {'primary': 'Valuables'})
        self.assertEqual(expected['44048']['authoring']['taxonomy'], {'primary': 'Taming Items'})
        self.assertNotIn('taxonomy', expected['43860']['authoring'])


if __name__ == '__main__':
    unittest.main()
