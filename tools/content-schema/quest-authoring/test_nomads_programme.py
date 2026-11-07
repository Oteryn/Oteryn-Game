"""Selected personal Nomads route retains exact rewards and explicit source witnesses."""
import json
from pathlib import Path
import unittest
import nomads_programme as n


class NomadsProgramme(unittest.TestCase):
    def test_ordered_three_camp_route_and_exact_separate_rewards(self):
        rows, objects = n.documents()
        self.assertEqual(len(rows), 6)
        self.assertEqual(len(objects['objects']), 3)
        for i, row in enumerate(rows):
            self.assertTrue(row['premium_required'])
            self.assertEqual(row['storage'], [dict(track=n.TRACK, minimum=i-1, maximum=i-1, set=i)])
            self.assertEqual(row['trigger'], 'use' if i % 2 else 'step_in')
        self.assertEqual(rows[1]['effects']['item_reward']['count'], 10)
        self.assertEqual(rows[3]['effects']['item_reward']['count'], 50)
        reward = rows[5]['effects']['item_reward']
        self.assertEqual((reward['item'], reward['count'], reward['contents']), ('oteryn:item.tibia.i7343', 1, []))
        self.assertEqual(reward['backpack_outputs'], [dict(item=f'oteryn:item.tibia.i{item}', count=count) for item,count in [(3032,3),(3027,1),(9057,1),(3054,1)]])
        self.assertTrue(rows[5]['completes'])
        self.assertEqual(rows[5]['effects']['achievement_reward'], 'oteryn:achievement/chest_robber')
        self.assertEqual(json.loads((n.ROOT/'content/quests/authored-world-objects.json').read_text()), objects)
        authored = json.loads((n.ROOT/'content/quests/authored-world-callbacks.json').read_text())['callbacks']
        self.assertEqual(sorted((r for r in authored if r['quest']==n.QUEST), key=lambda r:r['key']), sorted(rows,key=lambda r:r['key']))


if __name__ == '__main__':
    unittest.main()
