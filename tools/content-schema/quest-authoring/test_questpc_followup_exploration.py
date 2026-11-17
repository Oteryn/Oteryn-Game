"""Later native visits retain the actual survey and separate action/kill guards."""
from pathlib import Path
import unittest
from unittest import mock

import questpc_explore_bindings as explore

ROOT = Path(__file__).resolve().parents[3]
CASES = (
    ('dragon_corpse_quest', 'canary:reward-claim/kv/dragoncorpse', (32178,32224,-9,355), 1),
    ('dragon_tower_quest', 'canary:reward-claim/quest/u7_1/dragon_tower/rewards/backpack1',
     (33073,32169,-2,408), 10),
    ('edron_goblin_quest', 'canary:reward-claim/kv/edrongoblin1', (33096,31800,-10,353), 10),
)


class FollowupExplorationTests(unittest.TestCase):
    def test_each_followup_keeps_both_exact_predecessor_guards(self):
        for tail, claim, (x,y,floor,appearance), required in CASES:
            with self.subTest(quest=tail), mock.patch.object(explore, 'BINDINGS', (
                    ('oteryn:quest.'+tail, 's3', 'claim:'+claim, 'chosen-source/'+tail),)):
                callback, = explore.callbacks(ROOT)
                self.assertEqual(callback['selector']['native'], dict(
                    position=dict(x=x,y=y,floor=floor), ordinal=0,appearance=appearance))
                guards = {row['track'].rsplit('/',1)[-1]:row
                          for row in callback['storage'] if row.get('preserve')}
                self.assertEqual(set(guards), {'s1','s2'})
                self.assertEqual((guards['s1']['minimum'],guards['s1']['maximum']), (1,1))
                self.assertEqual((guards['s2']['minimum'],guards['s2']['maximum']),
                                 (required,required))
                self.assertEqual(callback['offerings'], [])
                self.assertIsNone(callback['teleport'])
                self.assertEqual(callback.get('admission'),
                                 {'premium':True} if tail=='edron_goblin_quest' else None)

    def test_all_three_followups_are_live(self):
        callbacks = explore.callbacks(ROOT)
        for tail, _, _, _ in CASES:
            with self.subTest(quest=tail):
                self.assertEqual(sum(c['quest']=='oteryn:quest.'+tail
                                     and any(s['track'].endswith('/s3')
                                             for s in c['storage'] if s.get('increment'))
                                     for c in callbacks),1)


if __name__ == '__main__':
    unittest.main()
