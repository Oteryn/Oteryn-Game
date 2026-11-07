import copy
import importlib.util
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PREFIX = 'oteryn:quest-reward/questpc3/threatened-animals/'

class ThreatenedAnimalSourceContracts(unittest.TestCase):
    def test_three_exact_source_exchanges(self):
        rows = {r['key'][len(PREFIX):]: r for r in json.loads((ROOT/'content/quests/shared-reward-callbacks.json').read_text())['callbacks'] if r['key'].startswith(PREFIX)}
        self.assertEqual(set(rows), {'ikassis-talisman', 'swan-feathers', 'stone-map'})
        talisman=rows['ikassis-talisman']
        self.assertEqual((talisman['topic'], talisman['keywords']), (1,['yes']))
        self.assertEqual(talisman['give'], {'item':'oteryn:item.tibia.i25295','count':1})
        self.assertEqual((talisman['storage'][0]['minimum'],talisman['storage'][0]['maximum'],talisman['storage'][0]['set']),(4,4,5))
        swan=rows['swan-feathers']
        self.assertEqual((swan['topic'],swan['keywords']),(0,['mission']))
        self.assertEqual(swan['take'],[{'item':'oteryn:item.tibia.i25244','count':5}])
        self.assertIsNone(swan['give'])
        self.assertEqual(swan['storage'][0]['set'],15)
        stone=rows['stone-map']
        self.assertEqual(stone['give'],{'item':'oteryn:item.tibia.i24945','count':1})
        self.assertEqual(len(stone['storage']),6)
        self.assertEqual([r['track'].rsplit('/',1)[-1] for r in stone['storage'][1:]],[f'stone{i}' for i in range(1,6)])
        self.assertTrue(all(r['minimum']==r['maximum']==r['set']==1 and r['preserve'] for r in stone['storage'][1:]))
        self.assertTrue(all(not r.get('completes',False) and len(r['sources'])==2 and 'git-blob=' in r['sources'][1] and 'sha256=' in r['sources'][1] for r in rows.values()))

    def test_talisman_admission_preserves_and_checks_core_facts(self):
        spec=importlib.util.spec_from_file_location('animal_item_admission',ROOT/'tools/content-migration/questpc3_item_admission.py')
        tool=importlib.util.module_from_spec(spec);spec.loader.exec_module(tool)
        rows=[r['definition'] for p in (ROOT/'content/items/definitions').glob('items-*.json') for r in json.loads(p.read_text())['records']]
        talisman=next(r for r in rows if r['identity']['key']=='oteryn:item.tibia.i25295')
        self.assertTrue(talisman['materializable']);self.assertEqual(talisman['stack_class'],'NonStackable')
        bad=copy.deepcopy(rows)
        next(r for r in bad if r['identity']['key']=='oteryn:item.tibia.i25295')['semantics']['physical']['value']['weight']['value']+=1
        with self.assertRaisesRegex(ValueError,'Threatened reward core facts changed'):
            tool.apply_admissions(bad)

if __name__=='__main__':unittest.main()
