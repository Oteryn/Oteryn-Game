import copy
import json
import unittest
from pathlib import Path
import builder
import effect_refinements as refine

HERE=Path(__file__).parent

class RefinementTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.state=json.loads((HERE/'main-state.json').read_text())
        cls.packet=json.loads((HERE/'refinements.json').read_text())
        cls.audit=json.loads((HERE/'audit.json').read_text())

    def test_full387_partition_and_actual5_changes(self):
        rows=self.audit['records']
        self.assertEqual(len(rows),387)
        self.assertEqual(len({r['transition_key'] for r in rows}),387)
        old=self.state['quests'];new=refine.apply_packet(old,self.packet)
        before={t['key']:t for q in old for t in q['transitions']}
        after={t['key']:t for q in new for t in q['transitions']}
        self.assertEqual(sum(before[k]!=after[k] for k in before),5)
        refused=sum(any(e['effect']['kind']=='COMPUTED' or not e['from_exact'] for e in t['effects']) for t in after.values())
        self.assertEqual(refused,382)
        for change in self.packet['changes']:
            expected=copy.deepcopy(before[change['transition_key']])
            expected['effects'][0]['effect']=change['effect']
            self.assertEqual(expected,after[change['transition_key']])
        self.assertEqual([q['tracks'] for q in old],[q['tracks'] for q in new])

    def test_guard_baseline_and_foreign_effect_rejected(self):
        quests=copy.deepcopy(self.state['quests']);key=self.packet['changes'][0]['transition_key']
        for q in quests:
            for t in q['transitions']:
                if t['key']==key:t['effects'][0]['from_exact']=False
        with self.assertRaises(ValueError):refine.apply_packet(quests,self.packet)
        packet=copy.deepcopy(self.packet);packet['changes'][0]['effect']={'kind':'SET','value':1}
        with self.assertRaises(Exception):refine.apply_packet(self.state['quests'],packet)

    def test_arithmetic_same_track_and_local_alias_boundaries(self):
        def name(s):return {'node_type':'Name','fields':{'id':s}}
        def invoke(method,receiver,args):return {'node_type':'Invoke','fields':{'func':name(method),'source':name(receiver),'args':args}}
        target=name('S')
        read=invoke('getStorageValue','player',[target])
        rhs={'node_type':'SubOp','fields':{'left':read,'right':{'node_type':'Number','fields':{'n':5}}}}
        write=invoke('setStorageValue','player',[target,rhs])
        self.assertEqual(builder.direct_add(write),{'kind':'ADD','value':-5})
        for value in (-1,0,1,5,2**31-1):self.assertEqual(value-5,value+builder.direct_add(write)['value'])
        wrong=copy.deepcopy(write);wrong['fields']['source']=name('boss')
        self.assertIsNone(builder.direct_add(wrong))
        wrong=copy.deepcopy(write);wrong['fields']['args'][0]=name('OTHER')
        self.assertIsNone(builder.direct_add(wrong))
        wrong=copy.deepcopy(write);wrong['fields']['args'][1]['fields']['left']=name('earlierSnapshot')
        self.assertIsNone(builder.direct_add(wrong))

    def test_source_proof_count_and_main_pin(self):
        self.assertEqual(sum(len(r['proofs']) for r in self.audit['records']),679)
        for row in self.audit['records']:
            self.assertEqual(builder.digest(row['baseline']),row['baseline_sha256'])
            self.assertTrue(row['proofs'])
            for proof in row['proofs']:
                self.assertRegex(proof['source_ref']['slice_sha256'],r'^[0-9a-f]{64}$')
                self.assertLess(proof['source_ref']['byte_start'],proof['source_ref']['byte_end_exclusive'])

if __name__=='__main__':unittest.main()
