"""M25 source room recipes retain literal combat and external trigger cells."""
import unittest

import build_activity_source_world as producer


def covers(regions, position):
    x, y, z = position
    return any(a <= x <= c and b <= y <= d and e <= z <= f
               for a, b, c, d, e, f in regions)


class M25SourceRooms(unittest.TestCase):
    def test_ahau_keeps_external_teleport_and_waiting_cells_separate(self):
        recipe = producer.ROOMS["galthen-ahau"]
        self.assertEqual(recipe["entry"], [34008, 31703, 10])
        self.assertEqual(recipe["regions"], [
            [33999, 31692, 34018, 31705, 10, 10],
            [34002, 31706, 34002, 31706, 10, 10],
            [34033, 31714, 34038, 31714, 10, 10],
            [34036, 31717, 34036, 31717, 10, 10],
        ])
        self.assertTrue(covers(recipe["regions"], [34002, 31706, 10]))
        self.assertTrue(covers(recipe["regions"], [34038, 31714, 10]))
        self.assertFalse(covers(recipe["regions"], [34020, 31710, 10]))
        self.assertFalse(covers(recipe["regions"], [34008, 31703, 11]))

    def test_mitmah_keeps_distinct_use_exit_and_destination(self):
        recipe = producer.ROOMS["galthen-mitmah-vanguard"]
        self.assertEqual(recipe["entry"], [34067, 31415, 11])
        self.assertEqual(recipe["regions"], [
            [34054, 31400, 34081, 31419, 11, 11],
            [34047, 31431, 34053, 31431, 11, 11],
            [34060, 31432, 34060, 31432, 11, 11],
        ])
        self.assertTrue(covers(recipe["regions"], [34068, 31418, 11]))
        self.assertTrue(covers(recipe["regions"], [34053, 31431, 11]))
        self.assertTrue(covers(recipe["regions"], [34060, 31432, 11]))
        self.assertFalse(covers(recipe["regions"], [34060, 31425, 11]))


if __name__ == "__main__":
    unittest.main()
