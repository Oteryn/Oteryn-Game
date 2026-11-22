"""Regressions for the registered native spell-family providers."""
import json
import unittest

from register_spell_families import REPOSITORY_PINS, ROOT, spell_import


class ArbazilothCorpseTests(unittest.TestCase):
    def test_registered_corpse_chain_uses_crystal_capacity_and_decay(self):
        path = ROOT / spell_import.PROVIDER_DESTINATIONS['item_profiles']
        profiles = json.loads(path.read_bytes())['records']
        for item, target in ((50029, 50028), (50028, 50027), (50027, None)):
            with self.subTest(item=item):
                key = f'oteryn:item.tibia.i{item}'
                row, = [r for r in profiles if r['production_definition']['production_key'] == key]
                self.assertEqual(row['admission'], {
                    'legal_destinations': ['Ground'], 'materializable': True,
                    'stack_class': 'NonStackable',
                })
                semantics = row['semantics']
                self.assertEqual(semantics['container'], (
                    {'state': 'KNOWN', 'value': {'capacity': {'state': 'KNOWN', 'value': 8}}}
                    if target else {'state': 'NOT_APPLICABLE'}))
                self.assertEqual(semantics['temporal']['value']['duration'],
                                 {'state': 'KNOWN', 'value': 60_000})
                self.assertEqual(semantics['temporal']['value']['decay_target'], (
                    {'state': 'KNOWN', 'value': {'key': f'oteryn:item.tibia.i{target}',
                                                 'revision': 'definition-r1'}}
                    if target else {'state': 'NOT_APPLICABLE'}))

    def test_loot_facts_use_the_physical_corpse_capacity(self):
        path = ROOT / REPOSITORY_PINS['loot_tables']
        facts, = [row for row in json.loads(path.read_bytes())['items']
                  if row['item']['key'] == 'oteryn:item.tibia.i50029']
        self.assertIs(facts['materializable'], True)
        self.assertEqual(facts['container_capacity'], 8)


if __name__ == '__main__':
    unittest.main()
