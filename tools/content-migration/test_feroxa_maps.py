"""Closed source-map admission for M52; no downloaded assets are published."""
import copy
import unittest
from pathlib import Path

import feroxa_maps

ROOT = Path(__file__).resolve().parents[2]


class FeroxaMapsTest(unittest.TestCase):
    def test_retained_maps_have_source_order_and_distinct_scenery(self):
        capture = feroxa_maps.load(ROOT)
        self.assertEqual([p['name'] for p in capture['phases']], ['middle', 'final'])
        self.assertEqual([len(p['tiles']) for p in capture['phases']], [1225, 1225])
        self.assertEqual([p['after_ms'] for p in capture['phases']], [1_800_000, 3_600_000])
        self.assertEqual(len(capture['palette']), 158)
        self.assertTrue(all(not p['pickupable'] and not p['container'] and not p['floor_change'] for p in capture['palette']))
        middle, final = capture['phases']
        self.assertEqual(sum(a != b for a, b in zip(middle['tiles'], final['tiles'])), 806)
        for phase in capture['phases']:
            self.assertEqual(phase['teleports'], [[33430, 31537, 11, 1, 33457, 31531, 11]])
            self.assertEqual({t[3] for t in phase['tiles']}, {0, 8})

    def test_substituted_capture_and_raw_maps_are_rejected(self):
        capture = feroxa_maps.load(ROOT)
        changed = copy.deepcopy(capture)
        changed['phases'][0]['tiles'][0][4][0] += 1
        with self.assertRaisesRegex(ValueError, 'FEROXA_MAP_CAPTURE_DIGEST'):
            feroxa_maps.verify_capture(feroxa_maps.encode(changed))
        with self.assertRaisesRegex(ValueError, 'FEROXA_MAP_SOURCE_DIGEST'):
            feroxa_maps.normalize(ROOT, 'middle', b'substituted')


if __name__ == '__main__':
    unittest.main()
