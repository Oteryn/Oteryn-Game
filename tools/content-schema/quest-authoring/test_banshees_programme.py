"""Selected Banshee route keeps source rewards and the charged amulet hold separate."""
import json
import unittest
import banshees_programme as b


class BansheesProgramme(unittest.TestCase):
    def test_seven_seals_and_escape_require_only_the_two_existing_rewards(self):
        world, npc, gates = b.documents()
        self.assertEqual(len(world), 8)
        self.assertEqual(len(npc), 1)
        self.assertTrue(all(row['minimum_level']==60 for row in world+npc))
        self.assertTrue(all(not row['effects'].get('item_reward') for row in world))
        self.assertIsNone(npc[0]['give'])
        self.assertEqual(npc[0]['storage'], [dict(track=b.TRACK,minimum=5,maximum=5,set=6)])
        self.assertEqual(npc[0]['npc'],'oteryn:npc.the_queen_of_the_banshees')
        self.assertEqual([row['storage'][0]['set'] for row in world], [0,1,2,3,4,5,7,8])
        self.assertEqual([row['trigger'] for row in world], ['step_in']*3+['use']*3+['step_in']*2)
        self.assertTrue(world[-1]['completes'])
        self.assertEqual(world[-1]['required_claims'], b.CLAIMS)
        self.assertEqual({row['claim'] for row in gates},set(b.CLAIMS))
        self.assertTrue(all(row['conditions']==[dict(track=b.TRACK,minimum=7,maximum=7)] for row in gates))
        self.assertNotIn('stone_skin_amulet',json.dumps(gates))

    def test_generated_runtime_documents_equal_the_source_programme(self):
        world,npc,gates=b.documents()
        actual=json.loads((b.ROOT/'content/quests/authored-world-callbacks.json').read_text())['callbacks']
        self.assertEqual(sorted([r for r in actual if r['key'].startswith('oteryn:quest-callback/authored-banshees-')],key=lambda r:r['key']),sorted(world,key=lambda r:r['key']))
        actual=json.loads((b.ROOT/'content/quests/shared-reward-callbacks.json').read_text())['callbacks']
        self.assertIn(npc[0],actual)
        actual=json.loads((b.ROOT/'content/quests/reward-claim-gates.json').read_text())['gates']
        for row in gates:self.assertIn(row,actual)


if __name__=='__main__':unittest.main()
