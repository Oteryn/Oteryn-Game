"""Brain Head callback coordinates require the pinned source-room recipe."""
import unittest
import build_activity_source_world as activity


class BrainHeadSourceRoom(unittest.TestCase):
    def test_brain_head_has_actual_donor_entry_and_global_heal_anchor(self):
        # Crystal201189f actions_portal_brain_head.lua:6,70-76 and
        # heal_brain_head.lua:2 are literal, unshifted source coordinates.
        room = activity.ROOMS.get("brain-head")
        self.assertIsNotNone(room, "the callback cannot borrow an unrelated synthetic frame")
        self.assertEqual(room["entry"], [31963, 32324, 10])
        for x, y in [(31954, 32325), (31953, 32324), (31955, 32324),
                     (31953, 32326), (31955, 32326), (31960, 32320),
                     (31960, 32330), (31947, 32320), (31947, 32330)]:
            self.assertTrue(any(a <= x <= c and b <= y <= d and e <= 10 <= f
                                for a, b, c, d, e, f in room["regions"]))


if __name__ == "__main__":
    unittest.main()
