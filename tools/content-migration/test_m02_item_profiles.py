"""The thieving squirrel's corpse and bait must support native death settlement."""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CORPSE = 'oteryn:item.tibia.i276'
BAIT = 'oteryn:item.tibia.i9843'


class M02ItemProfiles(unittest.TestCase):
    def test_registered_profiles_admit_corpse_and_carried_bait(self):
        document = json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())
        rows = {r['production_definition']['production_key']: r for r in document['records']}
        for key, destinations in ((CORPSE, ['Ground']), (BAIT, ['CharacterInventory', 'Ground'])):
            with self.subTest(item=key):
                self.assertTrue(key in rows, f'absent native profile: {key}')
                self.assertTrue(rows[key]['admission']['materializable'])
                self.assertEqual(rows[key]['admission']['legal_destinations'], destinations)
        temporal = rows[CORPSE]['semantics']['temporal']['value']
        self.assertEqual(temporal['duration'], {'state': 'KNOWN', 'value': 10_000})
        self.assertEqual(temporal['decay_target'], {'state': 'KNOWN', 'value': {
            'key': 'oteryn:item.tibia.i277', 'revision': 'definition-r1'}})
        self.assertEqual(rows[BAIT]['semantics']['physical']['value']['weight'],
                         {'state': 'KNOWN', 'value': 180})

    def test_gamel_letter_is_materializable_in_inventory(self):
        document = json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())
        rows = [r for r in document['records']
                if r['production_definition']['production_key'] == 'oteryn:item.tibia.i402']
        self.assertEqual(len(rows), 1, 'absent or duplicate native profile: i402')
        row, = rows
        self.assertTrue(row['admission']['materializable'])
        self.assertIn('CharacterInventory', row['admission']['legal_destinations'])
        self.assertEqual(row['semantics']['physical']['value']['weight'],
                         {'state': 'KNOWN', 'value': 50})

    def test_loot_facts_admit_corpse_and_bait(self):
        document = json.loads((ROOT / 'tools/content-schema/native-gameplay/loot-tables.json').read_bytes())
        facts = {r['item']['key']: r for r in document['items']}
        for key, capacity in ((CORPSE, 15), (BAIT, None)):
            with self.subTest(item=key):
                self.assertTrue(facts[key]['materializable'], f'inadmissible loot item: {key}')
                self.assertEqual(facts[key]['container_capacity'], capacity)


if __name__ == "__main__":
    unittest.main()
