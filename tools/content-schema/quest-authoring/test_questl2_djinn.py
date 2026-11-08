"""Djinn source counts are charges, never a stack of nonstackable amulets."""
import unittest

import qengine_rewards
import questl2_djinn


class DjinnNativeTradeFields(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.inputs = qengine_rewards.donor_inputs()

    def test_source_necklace_is_one_item_with_exact_full_charge_subtype(self):
        shops = questl2_djinn.shops(self.inputs, qengine_rewards.PIN)
        yaman = next(s for s in shops if s['npc'] == 'oteryn:npc.yaman')
        necklace = next(o for o in yaman['offers'] if
                        o['item']['key'] == 'oteryn:item.tibia.i3085' and
                        o['direction'] == 'SellToPlayer')
        self.assertEqual((necklace['count'], necklace['sub_type'], necklace['unit_price']),
                         (1, 200, 1000))

    def test_source_charge_mismatch_cannot_be_published_as_an_ordinary_stack(self):
        inputs = dict(self.inputs)
        path = 'data-global/npc/yaman.lua'
        inputs[path] = inputs[path].replace(b'count = 200', b'count = 199', 1)
        self.assertNotEqual(inputs[path], self.inputs[path])
        with self.assertRaisesRegex(ValueError, 'full-charge source mismatch'):
            questl2_djinn.shops(inputs, qengine_rewards.PIN)


if __name__ == '__main__':
    unittest.main()
