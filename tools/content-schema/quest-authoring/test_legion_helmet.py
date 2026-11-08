"""Selected contact gates preserve every previously accepted reward callback."""
import json
import unittest
from unittest.mock import patch
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
        gate,=[row for row in document['gates'] if row['claim']=='oteryn:reward-claim.kv.legiondawnport']
        self.assertEqual(gate['claim'],'oteryn:reward-claim.kv.legiondawnport')
        self.assertEqual(gate['quest'],'oteryn:quest.newhaven_legion_helmet_quest')
        self.assertEqual(gate['conditions'],[dict(track='oteryn:quest-progress/legion-helmet/'+name,minimum=1,maximum=1) for name in ['gustavo','viola','cave']])
        self.assertTrue(gate['sources'])

    def test_whole_route_has_exact_source_geometry_and_claim_gated_completion(self):
        rows=[r for r in json.loads(qengine_rewards.build()['content/quests/authored-world-callbacks.json'])['callbacks'] if r['quest']=='oteryn:quest.newhaven_legion_helmet_quest']
        self.assertEqual(len(rows),3)
        by_name={r['key'].removeprefix('oteryn:quest-callback/authored-'):r for r in rows}
        self.assertEqual(set(by_name),{'legion-route-entry','legion-cave-entry','legion-cave-return'})
        entry=by_name['legion-route-entry']
        self.assertEqual(entry['passages'],[dict(position=dict(x=32111,y=31903,floor=-7),appearance=593,replacement=594)])
        self.assertEqual(entry['teleport'],dict(x=32111,y=31904,floor=-7))
        cave=by_name['legion-cave-entry']
        self.assertEqual(cave['selector']['position'],dict(x=32111,y=31903,floor=-8))
        self.assertEqual(cave['selector']['appearance'],386)
        self.assertEqual(cave['teleport'],dict(x=32143,y=31909,floor=-8))
        self.assertEqual(cave['effects']['portals']['portals'],[
            dict(position=dict(x=32143,y=31909,floor=-8),destination=dict(x=32124,y=31939,floor=-8)),
            dict(position=dict(x=32124,y=31939,floor=-8),destination=dict(x=32143,y=31909,floor=-8))])
        finish=by_name['legion-cave-return']
        self.assertEqual(finish['selector']['position'],dict(x=32110,y=31937,floor=-8))
        self.assertEqual(finish['movement_origins'],[dict(x=x,y=y,floor=-8) for x,y in [(32122,31939),(32124,31939),(32123,31938),(32123,31940)]])
        self.assertEqual(finish['required_claims'],['oteryn:reward-claim.kv.legiondawnport'])
        self.assertTrue(finish['completes'])
        for r in rows:
            self.assertTrue(r['sources'][0].startswith('oteryn-authored:'))
            self.assertTrue(any(s.startswith('oteryn:selected-recipe:') for s in r['sources']))

    def test_prior_callbacks_preserved_exactly(self):
        old=[r for r in self.rows if not r['key'].startswith('oteryn:quest-reward/legion-contact-')]
        with patch('legion_helmet.extend', lambda rows: None):
            baseline=json.loads(qengine_rewards.build()['content/quests/shared-reward-callbacks.json'])['callbacks']
        self.assertEqual(old,baseline)

    def test_generated_registry_is_current(self):
        self.assertEqual(self.raw,(qengine_rewards.ROOT/'content/quests/shared-reward-callbacks.json').read_bytes())

if __name__=='__main__':unittest.main()
