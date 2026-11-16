import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def shard_definitions():
    for path in (ROOT / 'content/items/definitions').glob('items-*.json'):
        for row in json.loads(path.read_bytes())['records']:
            yield row['definition']


class HealingGlobalOverrides(unittest.TestCase):
    def test_potion_definitions_carry_the_global_values(self):
        expected = {
            'oteryn:item.tibia.i7642': [('HEALTH', 250, 350), ('MANA', 100, 200)],
            'oteryn:item.tibia.i23374': [('HEALTH', 420, 580), ('MANA', 150, 250)],
            'oteryn:item.tibia.i23375': [('HEALTH', 875, 1175)],
        }
        seen = {}
        for definition in shard_definitions():
            key = definition['identity']['key']
            semantics = definition.get('semantics', {})
            if key in expected:
                seen[key] = [(r['resource'], r['min'], r['max']) for r in
                             semantics['consumption']['value']['value']['restores']]
            if key == 'oteryn:item.tibia.i238':
                seen[key] = semantics['use_requirements']['value']['vocations']
        self.assertEqual(seen.pop('oteryn:item.tibia.i238'), {'state': 'NOT_APPLICABLE'})
        self.assertEqual(seen, expected)


if __name__ == '__main__':
    unittest.main()
