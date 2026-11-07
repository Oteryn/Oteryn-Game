import copy
import importlib.util
import json
import unittest
from pathlib import Path

ROOT=Path(__file__).resolve().parents[3]
PREFIX='oteryn:quest-transition/questpc3/threatened-rake/'
BASE='oteryn:quest-progress/quest/u11_40/threatened_dreams/mission04/'

class ThreatenedRakeContracts(unittest.TestCase):
    def test_five_retained_tool_positions_have_unique_source_flags_and_no_time_gate(self):
        rows=[r for r in json.loads((ROOT/'content/quests/world-callbacks.json').read_text())['callbacks'] if r['key'].startswith(PREFIX)]
        self.assertEqual(len(rows),5)
        positions=[(32617,31863),(32616,31864),(32615,31866),(32620,31864),(32619,31865)]
        for i,(row,(x,y)) in enumerate(zip(rows,positions),1):
            self.assertEqual(row['selector'],dict(position=dict(x=x,y=y,floor=-7),unique=None,action=None,appearance=None))
            self.assertEqual(row['trigger'],'use_carried_on_target')
            self.assertEqual(row['item_on_target'],dict(item='oteryn:item.tibia.i3452',count=1,replacement='oteryn:item.tibia.i3452',target_replacement=None))
            self.assertEqual(row['storage'],[dict(track=BASE+'grumpy_stone',minimum=1,maximum=1,set=1,preserve=True),dict(track=BASE+f'stone{i}',minimum=-1,maximum=0,set=1,preserve=False)])
            self.assertEqual(row['carried_use']['time_window'],dict(from_minute=0,to_minute=1440))
            self.assertFalse(row['passages']);self.assertFalse(row['offerings']);self.assertIsNone(row['teleport']);self.assertIsNone(row['lever'])
            self.assertIn('rake.lua:17-44 git-blob=fccb45af39905199322583cadeefbaf36282374d',row['sources'][1])
            self.assertIn('sha256=c3a636bdb1ed7a1ed0670692eab4d6c3e74f5239581c5cd2411f15a17f37fd61',row['sources'][1])

    def test_rake_native_admission_preserves_known_physical_facts(self):
        spec=importlib.util.spec_from_file_location('rake_admission',ROOT/'tools/content-migration/questpc3_item_admission.py')
        tool=importlib.util.module_from_spec(spec);spec.loader.exec_module(tool)
        rows=[r['definition'] for p in (ROOT/'content/items/definitions').glob('items-*.json') for r in json.loads(p.read_text())['records']]
        rake=next(r for r in rows if r['identity']['key']=='oteryn:item.tibia.i3452')
        self.assertTrue(rake['materializable']);self.assertEqual(rake['stack_class'],'NonStackable')
        self.assertEqual(rake['semantics']['physical']['value']['weight'],{'state':'KNOWN','value':1500})
        bad=copy.deepcopy(rows)
        next(r for r in bad if r['identity']['key']=='oteryn:item.tibia.i3452')['semantics']['physical']['value']['weight']['value']+=1
        with self.assertRaisesRegex(ValueError,'Threatened reward core facts changed'):tool.apply_admissions(bad)

if __name__=='__main__':unittest.main()
