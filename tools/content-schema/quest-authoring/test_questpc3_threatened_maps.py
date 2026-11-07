import json
import copy
import importlib.util
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PREFIX = 'oteryn:quest-reward/questpc3/threatened-maps/'


class ThreatenedMapSourceContracts(unittest.TestCase):
    def test_four_atomic_source_rewards_and_level_gate(self):
        rows = {row['key'][len(PREFIX):]: row for row in json.loads(
            (ROOT / 'content/quests/shared-reward-callbacks.json').read_text())['callbacks']
                if row['key'].startswith(PREFIX)}
        self.assertEqual(set(rows), {'tooth-gifts', 'tooth-map', 'tooth-collection', 'tree-map'})
        self.assertEqual(rows['tooth-gifts']['give'], {'item': 'oteryn:item.tibia.i25302', 'count': 3})
        self.assertEqual(rows['tooth-gifts']['minimum_level'], 30)
        self.assertEqual(rows['tooth-map']['minimum_level'], 30)
        self.assertEqual(rows['tooth-map']['take'], [{'item': 'oteryn:item.tibia.i25303', 'count': 3}])
        self.assertEqual(rows['tooth-map']['give'], {'item': 'oteryn:item.tibia.i24943', 'count': 1})
        collection = rows['tooth-collection']
        self.assertEqual(collection['take'], [{'item': f'oteryn:item.tibia.i{i}', 'count': 1}
                                             for i in [10196,22649,9685,5893,10275,22052]])
        self.assertEqual(collection['give'], {'item': 'oteryn:item.tibia.i3026', 'count': 6})
        self.assertEqual(collection['achievement'], 'oteryn:achievement/toothfairy_assistant')
        self.assertEqual((rows['tree-map']['topic'],rows['tree-map']['keywords']), (3,['seeds of life']))
        self.assertEqual(rows['tree-map']['give'], {'item': 'oteryn:item.tibia.i24944', 'count': 1})
        self.assertTrue(all(not row.get('completes',False) for row in rows.values()))

    def test_five_missing_teeth_are_portable_and_admitted(self):
        wanted = {10196,10275,22052,22649,5893}
        rows = {row['definition']['identity']['key']:row['definition']
                for p in (ROOT / 'content/items/definitions').glob('items-*.json')
                for row in json.loads(p.read_text())['records']}
        for ident in wanted:
            item = rows[f'oteryn:item.tibia.i{ident}']
            self.assertTrue(item['materializable'], f'tooth {ident} must support the native hand-in')
            self.assertEqual(item['stack_class'], 'NonStackable' if ident==22649 else 'StackCapable')


    def test_tooth_admission_rejects_contradictory_physical_and_stack_facts(self):
        path = ROOT / 'tools/content-migration/questpc3_item_admission.py'
        spec = importlib.util.spec_from_file_location('map_tooth_admission', path)
        tool = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(tool)
        rows = [row['definition'] for shard in (ROOT / 'content/items/definitions').glob('items-*.json')
                for row in json.loads(shard.read_text())['records']]
        for field in ('weight', 'stack'):
            bad = copy.deepcopy(rows)
            tooth = next(row for row in bad if row['identity']['key'] == 'oteryn:item.tibia.i10196')
            if field == 'weight':
                tooth['semantics']['physical']['value']['weight']['value'] += 1
            else:
                tooth['semantics']['stack']['value']['stackable']['value'] = False
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, 'tooth core facts changed'):
                tool.apply_admissions(bad)


if __name__ == '__main__':
    unittest.main()
