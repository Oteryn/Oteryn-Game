"""The complete capture rectangles, rather than their envelope, prove absence."""
import json
from pathlib import Path
import subprocess
import sys
import unittest

import build_activity_source_world as producer

ROOT = Path(__file__).resolve().parents[3]
BASELINE_INDEX = '--baseline-index' in sys.argv
if BASELINE_INDEX:
    sys.argv.remove('--baseline-index')


def document(name):
    path = f'content/encounters/{name}/source-room.json'
    raw = (subprocess.check_output(['git', 'show', ':' + path], cwd=ROOT)
           if BASELINE_INDEX else (ROOT / path).read_bytes())
    return json.loads(raw)


def covers(regions, position):
    x, y, z = position
    return any(a <= x <= c and b <= y <= d and e <= z <= f
               for a, b, c, d, e, f in regions)


class SourceRegions(unittest.TestCase):
    def test_every_capture_retains_exact_producer_regions_and_all_real_tiles(self):
        for name, recipe in producer.ROOMS.items():
            with self.subTest(room=name):
                capture = document(name)
                self.assertEqual(capture.get('source_regions'), recipe['regions'])
                for tile in capture['tiles']:
                    self.assertTrue(covers(capture['source_regions'], tile['source_position']))

    def test_anomaly_restoration_has_exact_source_item_policy(self):
        self.assertEqual([22894, 23049], producer.ROOMS["callback-anomaly"]["extra_items"])

    def test_primal_disjoint_gap_does_not_become_proved_absent(self):
        capture = document('primal')
        regions = capture.get('source_regions', [])
        self.assertTrue(covers(regions, [33565, 32758, 15]))
        self.assertTrue(covers(regions, [33520, 32871, 15]))
        self.assertFalse(covers(regions, [33530, 32800, 15]))
        a, b, c, d, e, f = capture['source_bounds']
        self.assertTrue(a <= 33530 <= c and b <= 32800 <= d and e <= 15 <= f)


if __name__ == '__main__':
    unittest.main()
