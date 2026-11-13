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

    def test_ichgahal_exit_keeps_raw_teleport_and_captures_its_exact_arrival(self):
        capture = document('rotten-blood-ichgahal')
        raw = next(tile for tile in capture['tiles']
                   if tile['source_position'] == [33004, 32343, 15])
        teleport = next(item for item in raw['items']
                        if item['server_item_id'] == 34111)
        self.assertEqual(teleport['depth'], 0)
        self.assertEqual(teleport['attributes'], {'dest': [34065, 31717, 15]})
        self.assertEqual(raw['semantics']['unmaterialized_dynamic_items'], [34111])
        arrivals = [tile for tile in capture['tiles']
                    if tile['source_position'] == teleport['attributes']['dest']]
        self.assertEqual(len(arrivals), 1)
        arrival = arrivals[0]
        self.assertEqual(arrival['items'], [
            {'server_item_id': 36344, 'depth': 0, 'attributes': {}},
            {'server_item_id': 292, 'depth': 0, 'attributes': {}},
        ])
        self.assertEqual(arrival['semantics']['ground'], 36344)
        self.assertEqual(arrival['semantics']['ground_speed'], 150)
        self.assertEqual(arrival['semantics']['unmaterialized_dynamic_items'], [])
        self.assertFalse(arrival['semantics']['flags']['block_solid'])
        regions = capture['source_regions']
        self.assertTrue(covers(regions, [34065, 31717, 15]))
        self.assertFalse(covers(regions, [34066, 31717, 15]))
        self.assertFalse(covers(regions, [33500, 32000, 15]))

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
