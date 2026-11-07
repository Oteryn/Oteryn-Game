"""D289 movable v2: exactly the v1 qualification less the held Native core Items."""

import unittest

import d289_holds
import lower_wiki_movable_d289_packet as v2


class MovableD289(unittest.TestCase):
    def test_only_the_held_item_moves_from_promotions_to_holds(self):
        base, packet = v2.v1.build(), v2.build()
        self.assertEqual(
            packet["counts"], {"promotions": 5691, "fields": 5691, "holds": 13}
        )
        self.assertEqual(packet["base_compiler"], base["compiler"])
        moved = [r for r in base["promotions"] if r not in packet["promotions"]]
        self.assertEqual(
            [r["item_key"] for r in moved], sorted(d289_holds.NATIVE_CORE_HOLD_KEYS)
        )
        self.assertEqual(packet["holds"][: len(base["holds"])], base["holds"])
        self.assertEqual(
            [h["reasons"] for h in packet["holds"][len(base["holds"]) :]],
            [[d289_holds.NATIVE_CORE_HOLD]],
        )


if __name__ == "__main__":
    unittest.main()
