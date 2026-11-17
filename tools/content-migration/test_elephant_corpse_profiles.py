"""The dead elephant corpse must be an admitted Ground container so elephant deaths settle."""
import json
import sys
import unittest
import elephant_corpse_profiles as corpse
ROOT = corpse.items.ROOT
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
from build_full_spell_artifact import native_corpse_capacity, native_loot_yield_facts

FIRST = 'oteryn:item.tibia.i6052'


class ElephantCorpse(unittest.TestCase):
    def test_source_chain_is_ground_only_with_capacity_30(self):
        records = corpse.capture(ROOT)['records']
        self.assertEqual([int(r['production_binding']['external_id']) for r in records], sorted(corpse.IDS))
        for row in records:
            self.assertEqual(row['admission']['legal_destinations'], ['Ground'])
            self.assertIsNone(native_loot_yield_facts(row['production_binding']['target'], row))
        first = next(r for r in records if r['production_definition']['production_key'] == FIRST)
        self.assertEqual(first['semantics']['temporal']['value']['duration'], {'state': 'KNOWN', 'value': 10000})
        self.assertEqual(native_corpse_capacity(first['production_binding']['target'], first), 30)

    def test_registered_native_profiles_admit_the_real_elephant_death(self):
        rows = {r['production_definition']['production_key']: r for r in json.loads(
            (ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())['records']}
        for item in corpse.IDS:
            self.assertIn(f'oteryn:item.tibia.i{item}', rows, f'absent native profile: {item}')
        first = rows[FIRST]
        self.assertEqual(native_corpse_capacity(first['production_binding']['target'], first), 30)

    def test_loot_facts_admit_the_corpse_for_both_elephants(self):
        facts = {r['item']['key']: r for r in json.loads(
            (ROOT / 'tools/content-schema/native-gameplay/loot-tables.json').read_bytes())['items']}
        self.assertEqual((facts[FIRST]['materializable'], facts[FIRST]['container_capacity']), (True, 30))


if __name__ == '__main__':
    unittest.main()
