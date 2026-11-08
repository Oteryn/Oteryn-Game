"""Physical custody must not manufacture a known speed for the mining-owned pick."""
import unittest
import questpc7_quest_item_reconcile as owner


def profile(item, attributes):
    return {'production_definition': {'production_key': f'oteryn:item.tibia.i{item}'},
            'attributes': attributes}


class ItemOwnerPreservationTests(unittest.TestCase):
    def test_absent_pick_speed_stays_absent(self):
        before, observed = profile(3456, {}), profile(3456, {'speed_bonus': 0})
        owner.preserve(before, observed)
        self.assertNotIn('speed_bonus', observed['attributes'])

    def test_explicit_unknown_pick_speed_stays_unknown(self):
        before, observed = profile(3456, {'speed_bonus': None}), profile(3456, {'speed_bonus': 0})
        owner.preserve(before, observed)
        self.assertIsNone(observed['attributes']['speed_bonus'])

    def test_known_pick_speed_is_not_silenced(self):
        before, observed = profile(3456, {'speed_bonus': 7}), profile(3456, {'speed_bonus': 0})
        owner.preserve(before, observed)
        self.assertEqual(observed['attributes']['speed_bonus'], 0)

    def test_other_custody_items_keep_their_observation(self):
        before, observed = profile(3577, {}), profile(3577, {'speed_bonus': 0})
        owner.preserve(before, observed)
        self.assertEqual(observed['attributes']['speed_bonus'], 0)


if __name__ == '__main__':
    unittest.main()
