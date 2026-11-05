import json
import unittest

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
        self.assertEqual([r['boss_id'] for r in actual['rooms'] if not r['qualified']], [])
        counts = {room["boss_id"]: room["connected_tiles"] for room in actual["rooms"]}
        # Independent donor OTBM count: Crystal 201189f data-global/world/world.otbm,
        # blob e61658ec0ab7b70ec35025ae969426d6fc57ae75; source-backed debris flags.
        # Distinct tiles grouped by lowest debris ID (overlaps counted only once):
        # 6475:48, 6476:7, 6477:10, 6478:6, 6479:9, 6480:10, 6481:12,
        # 6482:12, 6483:13, 6485:9, 6486:10 => 146; no debris:142 => 288.
        # One-off source check and full derivation: SPKG2-FIX-results.md.
        # samples/grizzly-room-count-evidence.md also explains the stale 73.
        self.assertEqual(
            {boss: counts[boss] for boss in (2, 6, 16)},
            {2: 90, 6: 64, 16: 288},
        )

if __name__ == '__main__':
    unittest.main()
