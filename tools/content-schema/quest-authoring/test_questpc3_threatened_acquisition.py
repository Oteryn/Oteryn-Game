import json
import unittest
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
KEY='oteryn:quest-transition/questpc3/threatened-acquisition/poacher-notes'
class AcquisitionSourceContracts(unittest.TestCase):
    def test_source_notes_are_aid_bound_once_and_do_not_advance_animals(self):
        rows=[r for r in json.loads((ROOT/'content/quests/world-callbacks.json').read_text())['callbacks'] if r['key']==KEY]
        self.assertEqual(len(rows),1);r=rows[0]
        self.assertEqual(r['trigger'],'use');self.assertEqual(r['selector'],dict(position=None,unique=None,action=20002,appearance=None))
        self.assertEqual(r['effects']['item_reward'],dict(item='oteryn:item.tibia.i25242',count=1,contents=[]))
        self.assertEqual([(s['track'].rsplit('/',1)[-1],s['minimum'],s['maximum'],s['set'],s.get('preserve',False)) for s in r['storage']],[('troubled_animals',7,7,7,True),('poacher_notes',-1,0,1,False)])
        self.assertEqual(r['offerings'],[]);self.assertIsNone(r['item_on_target']);self.assertFalse(r['reversible']);self.assertEqual(r['passages'],[])
        self.assertIn('git-blob=2814b532928717adb6630c84d384923f3d199328',r['sources'][-1])
    def test_existing_chest_claims_bind_exact_native_source_flags(self):
        records=[r['definition'] for p in (ROOT/'content/interactions/reward_claims').glob('reward-claim*.json') for r in json.loads(p.read_text())['records']]
        for suffix,uid,appearance,xyz,item in [('mission01.poacher_chest',6100,12764,dict(x=32787,y=31975,z=11),25235),('mission02.dark_moon_mirror',6101,25762,dict(x=33594,y=32214,z=9),25729)]:
            claim=next(r for r in records if r['identity']['key']=='oteryn:reward-claim.quest.u11_40.threatened_dreams.'+suffix)
            self.assertEqual(claim['readiness'],'ready');self.assertEqual(len(claim['placements']),1);p=claim['placements'][0]
            self.assertEqual(p['quest_transition'],'oteryn:quest-transition/quest/u11_40/threatened_dreams/'+suffix.replace('.','/')+'/chest')
            self.assertEqual(p['appearance_tibia_id'],appearance);self.assertEqual(p['source_binding']['project_position'],xyz)
            self.assertTrue(any(s['server']=='crystalserver' and s['unique_id']==uid for s in p['source_binding']['legacy_unique_ids']))
            self.assertEqual(p['reward']['items'],[dict(item=dict(family='Item',key=f'oteryn:item.tibia.i{item}',revision='definition-r1'),count=1)])
            self.assertEqual(claim['claim'],dict(per='character',repeat=dict(kind='once')))
if __name__=='__main__':unittest.main()
