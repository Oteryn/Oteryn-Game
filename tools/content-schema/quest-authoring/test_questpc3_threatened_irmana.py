import json
import unittest
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
NPC='oteryn:npc.irmana'
KEY='oteryn:quest-reward/questpc3/threatened-irmana/fur'
class IrmanaSourceContracts(unittest.TestCase):
    def test_exact_source_fee_and_owned_prerequisites(self):
        rows=[r for r in json.loads((ROOT/'content/quests/shared-reward-callbacks.json').read_text())['callbacks'] if r['key']==KEY]
        self.assertEqual(len(rows),1);r=rows[0]
        self.assertEqual((r['npc'],r['keywords'],r['topic'],r['next_topic']),(NPC,['yes'],8,0))
        self.assertEqual(r['fee'],dict(gold=1000,include_bank=False));self.assertEqual(r['take'],[])
        self.assertEqual(r['give'],dict(item='oteryn:item.tibia.i25238',count=1))
        self.assertEqual([(s['track'].rsplit('/',1)[-1],s['minimum'],s['maximum'],s['set'],s['preserve']) for s in r['storage']],[('troubled_animals',7,7,8,False),('poacher_notes',1,1,1,True)])
        self.assertIn('irmana.lua:208-218,247-253 git-blob=76f53aa5ae2e04daabf1a8cca74b63f3ad13e751',r['sources'][-1])
        self.assertFalse(r.get('completes',False));self.assertFalse(r.get('repeatable_check',False))
    def test_offer_only_opens_exact_source_confirmation_and_placement_is_admitted(self):
        offers=[r for r in json.loads((ROOT/'content/quests/shared-reward-offers.json').read_text())['offers'] if r['npc']==NPC]
        self.assertEqual(len(offers),1);r=offers[0]
        self.assertEqual((r['keywords'],r['topic'],r['next_topic']),(['fur'],0,8));self.assertTrue(all(s['preserve'] for s in r['storage']))
        self.assertIn('1000 gold',r['reply'][-1]);self.assertEqual(len(r['reply']),3)
        records=[r['declaration'] for p in (ROOT/'content/world/npc-placements').glob('placements-*.json') for r in json.loads(p.read_text())['records']]
        row=next(r for r in records if r['npc']==NPC);self.assertEqual(row['cell'],dict(x=32952,y=32109,floor=5))
if __name__=='__main__':unittest.main()
