import json
import unittest

import crystal_item_decay as decay


class CrystalItemDecayTests(unittest.TestCase):
    def test_every_donor_row_is_in_inventory(self):
        source = {identifier for identifier, row in decay.captured(decay.ROOT).items()
                  if row.get('inventory')}
        report = decay.read(decay.ROOT / decay.REPORT)
        reported = [row['id'] for row in report['records']]
        self.assertEqual(len(source), len(reported))
        self.assertEqual(source, set(reported))
        for row in report['records']:
            if row['after'] != 'qualified and runnable':
                self.assertTrue(row.get('gap'), row)

    def test_every_projectable_finite_chain_is_in_the_active_provider(self):
        active = decay.read(decay.ROOT / decay.PROFILES)
        _, missing = decay.project(decay.ROOT, active)
        self.assertEqual({}, missing)

    def test_source_torch_lamp_candle_and_ice_cream_have_exact_edges(self):
        profiles = decay.index_profiles(decay.read(decay.ROOT / decay.PROFILES))
        expected = {
            229: (120_000, None),
            2915: (180_000, 2916),
            2918: (240_000, 2919),
            2919: (60_000, None),
            2921: (600_000, 2923),
            2923: (300_000, 2925),
            2925: (300_000, 2926),
            2926: (300_000, None),
        }
        for identifier, (duration, target) in expected.items():
            with self.subTest(identifier=identifier):
                timed = decay.temporal(profiles[decay.name_key(identifier)])
                self.assertEqual(decay.K(decay.ABSOLUTE), timed['consumption_mode'])
                self.assertEqual(decay.K(duration), timed['duration'])
                self.assertEqual(decay.K(False), timed['stop_duration'])
                self.assertEqual(decay.NA if target is None else decay.K({
                    'key': decay.name_key(target), 'revision': 'definition-r1',
                }), timed['decay_target'])

    def test_decay_container_gap_is_tied_to_the_source_shape(self):
        source = decay.captured(decay.ROOT)[9151]
        self.assertTrue(source['container'])
        self.assertEqual((60_000, '0'), (decay.duration(source), source['decayTo']))
        report = json.loads((decay.ROOT / decay.REPORT).read_bytes())
        row = next(row for row in report['records'] if row['id'] == 9151)
        self.assertIn('atomic descendant retirement', row['gap'])


if __name__ == '__main__':
    unittest.main()
