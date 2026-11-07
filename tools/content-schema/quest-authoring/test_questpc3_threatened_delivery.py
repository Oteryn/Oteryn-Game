import json
import unittest
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
PREFIX='oteryn:quest-transition/questpc3/threatened-burn/'
class ThreatenedDeliveryContracts(unittest.TestCase):
    def test_three_exact_burn_variants_without_invented_position_or_period(self):
        rows={r['key'][len(PREFIX):]:r for r in json.loads((ROOT/'content/quests/world-callbacks.json').read_text())['callbacks'] if r['key'].startswith(PREFIX)}
        self.assertEqual(set(rows),{'book-pallet12648','book-pallet12649','whelp-fur'})
        for key,item,target,before,after in [('book-pallet12648',25235,12648,2,3),('book-pallet12649',25235,12649,2,3),('whelp-fur',25238,25237,8,9)]:
            row=rows[key]
            self.assertEqual(row['trigger'],'use_carried_on_target')
            self.assertEqual(row['selector'],dict(position=None,unique=None,action=None,appearance=target))
            self.assertEqual(row['item_on_target'],dict(item=f'oteryn:item.tibia.i{item}',count=1,replacement=None,target_replacement=None))
            self.assertEqual([(s['minimum'],s['maximum'],s['set'],s['preserve']) for s in row['storage']],[(before,before,after,False)])
            self.assertEqual(row['carried_use'],dict(actor_region=dict(minimum=dict(x=0,y=0,floor=-15),maximum=dict(x=65535,y=65535,floor=0)),time_window=dict(from_minute=0,to_minute=1440)))
            self.assertEqual(row['passages'],[]);self.assertEqual(row['offerings'],[]);self.assertIsNone(row['teleport']);self.assertIsNone(row['lever'])
            self.assertIn('git-blob=',row['sources'][-1]);self.assertIn('sha256=',row['sources'][-1])

    def test_existing_tent_callbacks_are_preserved_without_duplicate_authoring(self):
        rows=[r for r in json.loads((ROOT/'content/quests/world-callbacks.json').read_text())['callbacks'] if r['key'].startswith('oteryn:quest-callback/threatened-dreams-poacher-discovery-')]
        self.assertEqual(len(rows),3)
        self.assertEqual([r['selector']['position'] for r in rows],[dict(x=x,y=31811,floor=-7) for x in (32949,32950,32951)])
        self.assertTrue(all(r['trigger']=='step_in' and r['storage'][0]['minimum']==r['storage'][0]['maximum']==6 and r['storage'][0]['set']==7 for r in rows))
if __name__=='__main__':unittest.main()
