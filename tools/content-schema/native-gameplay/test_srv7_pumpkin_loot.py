"""Actual loot-builder admission of the four pinned Halloween reward profiles."""
from copy import deepcopy
import json
from pathlib import Path
import shutil
import tempfile
import unittest

import build_full_spell_artifact as build

ROOT = Path(__file__).resolve().parents[3]
SOURCE = ROOT / 'tools/content-schema/native-gameplay/pumpkin-skinning-item-profiles.json'
SHARDS = ('items-02500-02999.json', 'items-21500-21999.json',
          'items-31500-31999.json', 'items-32000-32499.json')


class PumpkinLootAdmission(unittest.TestCase):
    def facts(self, mutate=None):
        records = deepcopy(json.loads(SOURCE.read_bytes())['records'])
        if mutate:
            mutate(records)
        with tempfile.TemporaryDirectory(prefix='srv7-pumpkin-loot-') as directory:
            definitions = Path(directory)
            for name in SHARDS:
                shutil.copyfile(ROOT / 'content/items/definitions' / name, definitions / name)
            (definitions / 'spell-native-profiles.json').write_text(json.dumps({
                'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records}))
            return build.item_facts(definitions)

    def test_real_builder_admits_source_rewards_and_never_static_body(self):
        facts = self.facts()
        for identifier in (8032, 8178, 8177, 3594):
            self.assertTrue(facts[f'oteryn:item.tibia.i{identifier}']['materializable'], identifier)
        self.assertFalse(facts['oteryn:item.tibia.i12816']['materializable'])

    def test_non_pickupable_native_profile_cannot_promote_reference(self):
        def mutate(records):
            row = next(r for r in records if r['production_definition']['production_key']
                       == 'oteryn:item.tibia.i8032')
            row['semantics']['physical']['value']['pickupable'] = {'state': 'KNOWN', 'value': False}
        self.assertFalse(self.facts(mutate)['oteryn:item.tibia.i8032']['materializable'])

    def test_mismatched_native_binding_cannot_promote_reference(self):
        def mutate(records):
            row = next(r for r in records if r['production_definition']['production_key']
                       == 'oteryn:item.tibia.i8032')
            row['production_binding']['target']['key'] = 'oteryn:item.tibia.i12816'
        with self.assertRaisesRegex(ValueError, 'exact production Item binding'):
            self.facts(mutate)


if __name__ == '__main__':
    unittest.main()
