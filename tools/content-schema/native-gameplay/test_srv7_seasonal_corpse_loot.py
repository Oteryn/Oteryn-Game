"""Finite seasonal bodies must pass the actual native loot facts producer."""
from copy import deepcopy
import json
from pathlib import Path
import shutil
import tempfile
import unittest
import build_full_spell_artifact as build

ROOT = Path(__file__).resolve().parents[3]
SOURCE = ROOT / 'tools/content-schema/native-gameplay/seasonal-corpse-item-profiles.json'

class SeasonalCorpseAdmission(unittest.TestCase):
    def facts(self, mutate=None):
        records = deepcopy(json.loads(SOURCE.read_bytes())['records'])
        if mutate:
            mutate(records)
        with tempfile.TemporaryDirectory(prefix='srv7-seasonal-corpse-') as directory:
            definitions = Path(directory)
            for name in ('items-07000-07499.json', 'items-32000-32499.json', 'items-02500-02999.json', 'items-02000-02499.json', 'items-29500-29999.json'):
                shutil.copyfile(ROOT / 'content/items/definitions' / name, definitions / name)
            (definitions / 'spell-native-profiles.json').write_text(json.dumps({
                'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records}))
            return build.item_facts(definitions)

    def test_real_builder_admits_exact_two_ordinary_source_body_capacities(self):
        facts = self.facts()
        for identifier, capacity in ((8133, 15), (18114, 20)):
            self.assertEqual((facts[f'oteryn:item.tibia.i{identifier}']['materializable'],
                              facts[f'oteryn:item.tibia.i{identifier}']['container_capacity']), (True, capacity))
        self.assertFalse(facts['oteryn:item.tibia.i12816']['materializable'])

    def test_spike_stampor_actual_builder_admits_only_exact_source_body_capacities(self):
        facts = self.facts()
        for identifier, capacity in ((5963, 10), (12325, 32)):
            with self.subTest(identifier=identifier):
                fact = facts[f'oteryn:item.tibia.i{identifier}']
                self.assertEqual((fact['materializable'], fact['container_capacity']), (True, capacity))

    def test_non_ground_source_does_not_grant_corpse_admission(self):
        def mutate(rows):
            next(r for r in rows if r['production_definition']['production_key'] == 'oteryn:item.tibia.i8133')['admission']['legal_destinations'] = ['CharacterInventory']
        self.assertFalse(self.facts(mutate)['oteryn:item.tibia.i8133']['materializable'])

    def test_wrong_capacity_source_does_not_grant_corpse_admission(self):
        def mutate(rows):
            next(r for r in rows if r['production_definition']['production_key'] == 'oteryn:item.tibia.i18114')['semantics']['container']['value']['capacity']['value'] = 31
        self.assertFalse(self.facts(mutate)['oteryn:item.tibia.i18114']['materializable'])

if __name__ == '__main__':
    unittest.main()
