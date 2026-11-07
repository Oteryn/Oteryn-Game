import copy
import importlib.util
import json
import unittest
from pathlib import Path
import questpc3_threatened_inventory as authoring


class ThreatenedInventoryAuthoring(unittest.TestCase):
    def test_source_costs_rewards_and_once_flags(self):
        callbacks = {row['key'].rsplit('/', 1)[-1]: row for row in authoring.documents()['content/quests/shared-reward-callbacks.json']['callbacks']
                     if row['key'].startswith(authoring.PREFIX)}
        self.assertEqual(set(callbacks), {'aurita-vial', 'taegen-catcher', 'valindara-cloak'})
        vial = callbacks['aurita-vial']
        self.assertEqual(vial['take'], [])
        self.assertEqual(vial['give'], {'item': 'oteryn:item.tibia.i25731', 'count': 1})
        self.assertEqual(vial['keywords'], ['starlight vial', 'empty starlight vial'])
        self.assertEqual((vial['topic'], vial['storage'][0]['minimum'], vial['storage'][0]['maximum'], vial['storage'][0]['set']), (0, -1, 0, 1))
        catcher = callbacks['taegen-catcher']
        self.assertEqual(catcher['take'], [{'item': f'oteryn:item.tibia.i{ident}', 'count': 2} for ident in [675, 676, 677, 678]])
        self.assertEqual(catcher['give'], {'item': 'oteryn:item.tibia.i25733', 'count': 1})
        self.assertEqual((catcher['topic'], catcher['storage'][0]['set']), (3, 1))
        cloak = callbacks['valindara-cloak']
        self.assertEqual(cloak['take'], [{'item': 'oteryn:item.tibia.i26181', 'count': 100}])
        self.assertEqual(cloak['give'], {'item': 'oteryn:item.tibia.i25779', 'count': 1})
        self.assertEqual((cloak['topic'], cloak['storage'][0]['minimum'], cloak['storage'][0]['set']), (1, 1, 2))
        for row in callbacks.values():
            self.assertEqual(row['storage'][1], {'track': 'oteryn:quest-progress/quest/u11_40/threatened_dreams/mission03_1', 'minimum': 4, 'maximum': 4, 'set': 4, 'preserve': True})
            self.assertFalse(row.get('completes', False), 'optional exchanges must not close the whole story')

    def test_foreign_programs_are_preserved(self):
        old = json.loads((authoring.ROOT / 'content/quests/shared-reward-callbacks.json').read_text())
        new = authoring.documents()['content/quests/shared-reward-callbacks.json']
        for row in old['callbacks']:
            if not row['key'].startswith(authoring.PREFIX):
                self.assertIn(row, new['callbacks'])

    def test_portable_reward_admissions_preserve_all_other_facts(self):
        tool = authoring.ROOT / 'tools/content-migration/questpc3_item_admission.py'
        spec = importlib.util.spec_from_file_location('questpc3_admission', tool)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        rows = [row['definition'] for shard in sorted((authoring.ROOT / 'content/items/definitions').glob('items-*.json')) for row in json.loads(shard.read_text())['records']]
        wanted = {'oteryn:item.tibia.i25731', 'oteryn:item.tibia.i25733', 'oteryn:item.tibia.i25779'}
        before = {row['identity']['key']: copy.deepcopy(row) for row in rows if row['identity']['key'] in wanted}
        module.apply_admissions(rows)
        for row in rows:
            key = row['identity']['key']
            if key in wanted:
                expected = before[key]
                expected['materializable'] = True
                expected['stack_class'] = 'NonStackable'
                self.assertEqual(row, expected)
        bad = copy.deepcopy(rows)
        next(row for row in bad if row['identity']['key'] == 'oteryn:item.tibia.i25731')['semantics']['physical']['value']['weight']['value'] += 1
        with self.assertRaises(ValueError):
            module.apply_admissions(bad)


if __name__ == '__main__':
    unittest.main()
