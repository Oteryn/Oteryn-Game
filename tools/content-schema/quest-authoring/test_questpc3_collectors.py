import unittest
import questpc3_collectors as source


class CollectorDelivery(unittest.TestCase):
    def test_pilgrimage_requires_six_blessers_in_order_before_twenty_platinum(self):
        quest = source.build_pilgrimage()['quests'][0]
        self.assertEqual(quest['quest'], 'oteryn:quest.pilgrimage_of_ashes')
        self.assertEqual(quest['admission'], {'minimum_level': 0, 'premium': True})
        expected = ['guide_jonathan', 'norf', 'humphrey', 'edala', 'kawill', 'pydar', 'eremo', 'guide_jonathan']
        self.assertEqual(len(quest['stages']), 8)
        for index, (stage, npc) in enumerate(zip(quest['stages'], expected)):
            callback = stage['trigger']['callback']
            self.assertEqual(callback['npc'], 'oteryn:npc.' + npc)
            self.assertEqual(callback['storage'], [{'track': quest['tracks'][0]['key'], 'minimum': index, 'maximum': index, 'set': index + 1}])
            self.assertEqual(stage['completes'], index == 7)
            self.assertEqual(callback['completes'], index == 7)
            self.assertEqual(callback['take'], [])
            self.assertIsNone(callback['achievement'])
            if index == 7:
                self.assertEqual(callback['give'], {'item': 'oteryn:item.tibia.i3035', 'count': 20, 'contents': [], 'premium': True})
            else:
                self.assertIsNone(callback['give'])


if __name__ == '__main__':
    unittest.main()
