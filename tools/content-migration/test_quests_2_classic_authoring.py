"""Source-owned classic quest data must keep guarded mutations and deferred rewards apart."""
import json
import unittest

import quests_2_classic_authoring as authoring


class ClassicAuthoring(unittest.TestCase):
    def read(self, path):
        return json.loads((authoring.ROOT / path).read_bytes())

    def test_morris_starters_keep_all_flags_and_no_reward_only_completion(self):
        dawn = self.read('content/quests/dawnport-starts.json')['quests'][0]
        transitions = {row['key'].rsplit('/', 1)[1]: row for row in dawn['transitions']}
        self.assertEqual(len(transitions), 8)
        self.assertNotIn('return-the-lost-amulet', transitions)
        self.assertNotIn('return-the-rare-herb', transitions)
        herb = transitions['start-the-rare-herb']
        self.assertEqual([(e['track'].rsplit('/', 1)[1], e['effect']['value'])
                          for e in herb['effects']], [('the_rare_herb', 1), ('herb_flower', 1)])
        self.assertEqual(herb['effects'][0]['from'], {'op': 'LT', 'value': 1})
        deferred = self.read('content/dialogues/quests-2/classic-deferred-exchanges.json')
        self.assertEqual(deferred['disposition'], 'LEFT_OUT_SHARED_OWNER')
        self.assertTrue(deferred['answer'].endswith('#issuecomment-6589521017'))
        self.assertEqual(len(deferred['rules']), 2)

    def test_disrepair_keeps_crystal_track_identity_and_exact_three_effects(self):
        q = self.read('content/quests/rottin-wood-starts.json')['quests'][0]
        self.assertEqual(q['quest'], authoring.ROTTIN_QUEST)
        self.assertEqual(len(q['transitions']), 1)
        step = q['transitions'][0]
        self.assertEqual(step['effects'][0]['from'], {'op': 'EQ', 'value': 2})
        self.assertEqual([(e['track'], e['effect']['value']) for e in step['effects']], [
            (authoring.ROTTIN + 'questline', 3),
            (authoring.ROTTIN + 'mission02', 1), (authoring.WALLCOUNT, 0)])
        self.assertEqual(step['source']['revision'], authoring.PIN)
        self.assertEqual(step['source']['line'], 176)
        self.assertFalse(step['completes'])

    def test_every_runtime_reply_is_the_full_literal_source_branch(self):
        source = self.read('content/dialogues/quests-2/classic-donor-dialogues.json')
        captured = {(npc['npc'], branch['key']): branch for npc in source['npcs']
                    for branch in npc['branches']}
        native = self.read('content/dialogues/classic-quest-branches.json')
        for npc in native['npcs']:
            for branch in npc['branches']:
                original = captured[(npc['npc'], branch['key'])]
                self.assertTrue(original['literal'])
                self.assertEqual(branch['reply'], original['source_text_parts'])
        rottin = next(npc for npc in native['npcs'] if npc['npc'] == 'oteryn:npc.rottin_wood')
        offer = next(branch for branch in rottin['branches'] if branch['key'] == 'line-98')
        self.assertEqual(len(offer['reply']), 5)
        self.assertIn('tear down half the forest', offer['reply'][2])


if __name__ == '__main__':
    unittest.main()
