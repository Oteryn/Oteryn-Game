import json
import copy
import importlib.util
import unittest
from pathlib import Path

ROOT=Path(__file__).resolve().parents[3]
PREFIX='oteryn:quest-reward/questpc3/threatened-final/'


class ThreatenedFinalContracts(unittest.TestCase):
    def test_four_source_handins_and_exact_final_guards(self):
        rows={r['key'][len(PREFIX):]:r for r in json.loads((ROOT/'content/quests/shared-reward-callbacks.json').read_text())['callbacks'] if r['key'].startswith(PREFIX)}
        self.assertEqual(set(rows),{'taegen-flower','taegen-herb','maelyrra-held-lights','maelyrra-blossom-bag'})
        self.assertEqual(rows['taegen-flower']['take'],[{'item':'oteryn:item.tibia.i25782','count':1}])
        self.assertEqual(rows['taegen-flower']['keywords'],['mission'])
        self.assertIsNone(rows['taegen-flower']['give'])
        self.assertEqual(rows['taegen-herb']['take'],[{'item':'oteryn:item.tibia.i5953','count':1}])
        self.assertEqual(rows['taegen-herb']['keywords'],['yes'])
        self.assertEqual(rows['taegen-herb']['next_topic'],3)
        held=rows['maelyrra-held-lights']
        self.assertEqual(held['present'],[{'item':f'oteryn:item.tibia.i{i}','count':1} for i in [25734,25732,25730]])
        self.assertEqual(held['take'],[])
        self.assertIsNone(held['give'])
        self.assertEqual(held['topic'],7)
        bag=rows['maelyrra-blossom-bag']
        self.assertEqual(bag['topic'],8)
        self.assertEqual(bag['give'],{'item':'oteryn:item.tibia.i25780','count':1})
        charges=[r for r in bag['storage'] if 'charged_' in r['track']]
        self.assertEqual(len(charges),3)
        self.assertTrue(all((r['minimum'],r['maximum'],r['set'],r['preserve'])==(0,0,0,True) for r in charges))
        self.assertTrue(all(not r.get('completes',False) for r in rows.values()))

    def test_six_portable_operands_and_bag_capacity_are_admitted(self):
        ids={25730,25732,25734,25780,25782,5953}
        found=set()
        for p in (ROOT/'content/items/definitions').glob('items-*.json'):
            for row in json.loads(p.read_text())['records']:
                d=row['definition'];key=d['identity']['key']
                if key not in {f'oteryn:item.tibia.i{i}' for i in ids}:continue
                found.add(key)
                self.assertTrue(d['materializable'],key)
                self.assertEqual(d['stack_class'],'StackCapable' if key.endswith('i5953') else 'NonStackable')
                if key.endswith('i25780'):
                    self.assertEqual(d['semantics']['container'],{'state':'KNOWN','value':{'capacity':{'state':'KNOWN','value':24}}})
        self.assertEqual(len(found),6)


    def test_bag_capacity_and_herb_stack_contradictions_are_rejected(self):
        path=ROOT/'tools/content-migration/questpc3_item_admission.py'
        spec=importlib.util.spec_from_file_location('final_item_admission',path)
        tool=importlib.util.module_from_spec(spec);spec.loader.exec_module(tool)
        rows=[r['definition'] for p in (ROOT/'content/items/definitions').glob('items-*.json') for r in json.loads(p.read_text())['records']]
        for key,field in [('oteryn:item.tibia.i25780','container'),('oteryn:item.tibia.i5953','stack')]:
            bad=copy.deepcopy(rows);d=next(r for r in bad if r['identity']['key']==key)
            if field=='container':d['semantics']['container']['value']['capacity']['value']=25
            else:d['semantics']['stack']['value']['stackable']['value']=False
            with self.subTest(key=key),self.assertRaises(ValueError):tool.apply_admissions(bad)


if __name__=='__main__':unittest.main()
