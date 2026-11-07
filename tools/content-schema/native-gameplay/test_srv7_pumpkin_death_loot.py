"""Current event common loot must survive the actual native section producer."""
import json
from copy import deepcopy
import srv7_pumpkin_death_loot as pumpkin
import unittest
import build_full_spell_artifact as build

class PumpkinCurrentDeathLoot(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        profiles = json.loads((build.CREATURE_DEFINITIONS / 'spell-native-profiles.json').read_bytes())
        records = [r for r in profiles['records'] if r['profile']['target']['key'] == 'oteryn:creature.the_mutated_pumpkin']
        assert len(records) == 1
        cls.section = json.loads(build.loot_tables({'records': records}))

    def test_current_common_quantities_in_real_native_loot_section(self):
        table = next(t for t in self.section['tables'] if t['identity']['key'] == 'oteryn:loot.creature.the_mutated_pumpkin')
        for identifier, quantity in ((3594, 5), (8177, 10)):
            with self.subTest(identifier=identifier):
                entry = next(e for e in table['entries'] if e['item']['key'] == f'oteryn:item.tibia.i{identifier}')
                self.assertEqual((entry['min_count'], entry['max_count'], entry['probability_ppm']), (quantity, quantity, 1000000))

    def test_every_actual_pumpkin_loot_operand_is_materializable(self):
        table = next(t for t in self.section['tables'] if t['identity']['key'] == 'oteryn:loot.creature.the_mutated_pumpkin')
        for entry in table['entries']:
            with self.subTest(item=entry['item']['key']):
                fact = next(f for f in self.section['items'] if f['item'] == entry['item'])
                self.assertTrue(fact['materializable'])

    def test_source_stackable_sweets_retain_real_quantities(self):
        for identifier in (3599,6569,6574):
            with self.subTest(identifier=identifier):
                fact = next(f for f in self.section['items'] if f['item']['key'] == f'oteryn:item.tibia.i{identifier}')
                self.assertTrue(fact.get('stackable',False))

    def test_rare_reference_cannot_bypass_missing_native_inventory_authority(self):
        native = json.loads((build.ITEM_DEFINITIONS / 'spell-native-profiles.json').read_bytes())
        for identifier in (6491,6571):
            row = deepcopy(next(r for r in native['records'] if r['production_definition']['production_key'] == f'oteryn:item.tibia.i{identifier}'))
            target = row['production_binding']['target']
            row['admission']['legal_destinations'] = ['Ground']
            facts = {target['key']: {'item': target, 'materializable': False}}
            pumpkin.admit_stacks(facts, {'records':[row]})
            self.assertFalse(facts[target['key']]['materializable'])

    def test_current_common_quantities_retain_source_native_stack_admission(self):
        for identifier in (3594, 8177):
            with self.subTest(identifier=identifier):
                fact = next(f for f in self.section['items'] if f['item']['key'] == f'oteryn:item.tibia.i{identifier}')
                self.assertTrue(fact.get('stackable', False))

if __name__ == '__main__':
    unittest.main()
