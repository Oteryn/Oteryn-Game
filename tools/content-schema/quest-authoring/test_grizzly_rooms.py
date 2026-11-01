import json
import unittest
from pathlib import Path
import grizzly_rooms

class GrizzlyRoomTests(unittest.TestCase):
    def test_generated_rooms_match_map_and_preserve_both_source_positions(self):
        actual = grizzly_rooms.generate()
        self.assertEqual(actual, json.loads(grizzly_rooms.OUTPUT.read_text()))
        source = json.loads((grizzly_rooms.ROOT / 'content/quests/killing-in-the-name-of.json').read_text())
        for room, boss in zip(actual['rooms'], source['bosses'], strict=True):
            self.assertEqual(room['boss_id'], boss['id'])
            if room['qualified']:
                left, top, right, bottom, z = room['bounds']
                for position in [boss['player_position'], boss['boss_position']]:
                    self.assertEqual(position['z'], z)
                    self.assertTrue(left <= position['x'] <= right)
                    self.assertTrue(top <= position['y'] <= bottom)
            else:
                self.assertIsNone(room['bounds'])
                self.assertTrue(room['reason'])
        self.assertEqual([r['boss_id'] for r in actual['rooms'] if not r['qualified']], [2,6,16])

if __name__ == '__main__':
    unittest.main()
