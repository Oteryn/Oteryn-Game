"""Actual native item_facts producer, all source Demon/Stampor loot operands, no roll edits."""
from copy import deepcopy
from functools import lru_cache
import json
from pathlib import Path
import shutil
import tempfile
import unittest
import build_full_spell_artifact as build
ROOT = Path(__file__).resolve().parents[3]
DEFINITIONS = ROOT / 'content/items/definitions'
PROMOTED = {3415, 3062, 9647, 12312, 12313, 12314}
STACKED = {3062, 3078, 3030, 3027, 268, 3287, 266, 9647, 12314, 236, 237, 12312, 12313, 3035}
@lru_cache(maxsize=1)
def source():
    tables = json.loads((ROOT / 'tools/content-schema/native-gameplay/loot-tables.json').read_bytes())['tables']
    operands = {entry['item']['key'] for table in tables if table['identity']['key'] in
                {'oteryn:loot.creature.demon_skeleton', 'oteryn:loot.creature.stampor'} for entry in table['entries']}
    shards = tuple(path for path in DEFINITIONS.glob('items-*.json') if any(
        record['definition']['identity']['key'] in operands
        for record in json.loads(path.read_bytes()).get('records', [])))
    profiles = json.loads((DEFINITIONS / 'spell-native-profiles.json').read_bytes())
    records = [row for row in profiles['records'] if row['production_definition']['production_key'] in operands]
    if len(records) != len(operands):
        raise ValueError('Every potential source operand needs an already admitted native profile')
    return operands, shards, records
class SourceLootAdmission(unittest.TestCase):
    def facts(self, mutate=None):
        operands, shards, records = source()
        records = deepcopy(records)
        if mutate:
            mutate(records)
        with tempfile.TemporaryDirectory(prefix='srv7-spike-stampor-loot-') as directory:
            definitions = Path(directory)
            for shard in shards:
                shutil.copyfile(shard, definitions / shard.name)
            (definitions / 'spell-native-profiles.json').write_text(json.dumps({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records}))
            facts = build.item_facts(definitions)
            return {key: facts[key] for key in operands}
    def test_actual_builder_admits_every_potential_source_operand(self):
        facts = self.facts()
        for key, fact in facts.items():
            with self.subTest(key=key):
                self.assertTrue(fact['materializable'])
    def test_actual_builder_preserves_all_fourteen_native_stack_classes(self):
        facts = self.facts()
        for identifier in STACKED:
            with self.subTest(identifier=identifier):
                self.assertTrue(facts[f'oteryn:item.tibia.i{identifier}'].get('stackable', False))
        self.assertFalse(facts['oteryn:item.tibia.i3415'].get('stackable', False))
    def test_wrong_destination_does_not_promote_source_shield(self):
        def mutate(rows):
            next(row for row in rows if row['production_definition']['production_key'] == 'oteryn:item.tibia.i3415')['admission']['legal_destinations'] = ['Ground']
        self.assertFalse(self.facts(mutate)['oteryn:item.tibia.i3415']['materializable'])
    def test_foreign_binding_is_not_treated_as_the_source_item(self):
        def mutate(rows):
            next(row for row in rows if row['production_definition']['production_key'] == 'oteryn:item.tibia.i9647')['production_binding']['target']['revision'] = 'definition-r2'
        with self.assertRaises(ValueError):
            self.facts(mutate)
if __name__ == '__main__':
    unittest.main()
