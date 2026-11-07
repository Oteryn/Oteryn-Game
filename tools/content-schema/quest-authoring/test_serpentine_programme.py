"""The selected tower trial gates the existing reward without a parallel mint."""
import json
import unittest
import serpentine_programme as s


class SerpentineProgramme(unittest.TestCase):
    def test_ordered_trial_and_source_pot_route_share_one_existing_reward(self):
        rows, gate = s.documents()
        self.assertEqual(len(rows), 6)
        by_name = {r['key'].removeprefix('oteryn:quest-callback/authored-serpentine-'): r for r in rows}
        for i, name in enumerate(['explore', 'torch', 'lever', 'observation']):
            row = by_name[name]
            self.assertEqual(row['storage'], [dict(track=s.TRACK, minimum=i-1, maximum=i-1, set=i)])
            self.assertTrue(row['premium_required'])
            self.assertFalse(row['effects'].get('item_reward'))
            self.assertEqual(row['quest'], s.QUEST)
        self.assertTrue(by_name['observation']['completes'])
        self.assertEqual(by_name['torch']['passages'], [dict(position=s.p(33151, 32866, 8), appearance=1345, replacement=None)])
        self.assertEqual(by_name['lever']['passages'], [dict(position=s.p(x,y,9), appearance=2129, replacement=None) for x in [33148,33149] for y in [32867,32868]])
        entry = by_name['entry']
        self.assertTrue(entry['storage'][0]['read_only'])
        self.assertEqual(entry['teleport'], s.p(33145,32863,7))
        self.assertEqual(entry['remote_teleport'], dict(position=s.p(33145,32862,7), appearance=3465, destination=s.p(33151,32864,7)))
        self.assertEqual(by_name['return']['teleport'], s.p(33147,32864,7))
        self.assertEqual(gate['claim'], 'oteryn:reward-claim.quest.u7_3.serpentine_tower.white_pearl')
        self.assertEqual(gate['conditions'], [dict(track=s.TRACK,minimum=3,maximum=3)])
    def test_generated_callbacks_and_gate_match_the_source_programme(self):
        rows, gate = s.documents()
        authored=json.loads((s.ROOT/'content/quests/authored-world-callbacks.json').read_text())['callbacks']
        self.assertEqual(sorted([r for r in authored if r['quest']==s.QUEST],key=lambda r:r['key']), sorted(rows,key=lambda r:r['key']))
        gates=json.loads((s.ROOT/'content/quests/reward-claim-gates.json').read_text())['gates']
        self.assertIn(gate,gates)


if __name__ == '__main__':
    unittest.main()
