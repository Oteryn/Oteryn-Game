"""Selected contact gates preserve every previously accepted reward callback."""
import hashlib
import json
import unittest
import qengine_rewards

class LegionContactsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.raw=qengine_rewards.build()['content/quests/shared-reward-callbacks.json']
        cls.rows=json.loads(cls.raw)['callbacks']

    def test_both_contacts_have_separate_once_only_tracks_without_rewards(self):
        rows=[r for r in self.rows if r['key'].startswith('oteryn:quest-reward/legion-contact-')]
        self.assertEqual(len(rows),2)
        self.assertEqual({r['npc'] for r in rows},{'oteryn:npc.gustavo_the_guard','oteryn:npc.viola'})
        for r in rows:
            self.assertEqual(r['storage'],[dict(track='oteryn:quest-progress/legion-helmet/'+r['key'].rsplit('-',1)[1],minimum=-1,maximum=-1,set=1)])
            self.assertIsNone(r['give'])
            self.assertEqual(r['take'],[])
            self.assertNotIn('minimum_level',r)
            self.assertNotIn('experience',r)
            self.assertNotIn('completes',r)

    def test_claim_gate_is_bounded_to_the_two_exact_personal_contacts(self):
        raw=qengine_rewards.build()['content/quests/reward-claim-gates.json']
        document=json.loads(raw)
        self.assertEqual(document['schema'],1)
        gate,=document['gates']
        self.assertEqual(gate['claim'],'oteryn:reward-claim.kv.legiondawnport')
        self.assertEqual(gate['quest'],'oteryn:quest.newhaven_legion_helmet_quest')
        self.assertEqual(gate['conditions'],[dict(track='oteryn:quest-progress/legion-helmet/'+name,minimum=1,maximum=1) for name in ['gustavo','viola']])
        self.assertTrue(gate['sources'])

    def test_prior_callbacks_preserved_exactly(self):
        old=[r for r in self.rows if not r['key'].startswith('oteryn:quest-reward/legion-contact-')]
        self.assertEqual(hashlib.sha256(json.dumps(old,sort_keys=True,separators=(',',':')).encode()).hexdigest(),'9213a6522d1ab4673f25c6e635d04c8a4e093dce77b43ce0ebf63c72f2cf94cb')

    def test_generated_registry_is_current(self):
        self.assertEqual(self.raw,(qengine_rewards.ROOT/'content/quests/shared-reward-callbacks.json').read_bytes())

if __name__=='__main__':unittest.main()
