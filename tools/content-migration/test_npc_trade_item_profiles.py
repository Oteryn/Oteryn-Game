"""NPCFIX: source stock must also have an inventory policy in the active provider."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
KEY = 'oteryn:item.tibia.i23986'


class NpcTradeItemProfiles(unittest.TestCase):
    def test_albinius_tome_keeps_the_existing_canonical_facts_and_binding(self):
        profiles = json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())['records']
        matches = [r for r in profiles if r['production_definition']['production_key'] == KEY]
        self.assertEqual(len(matches), 1, 'Albinius needs the real active Item policy')
        profile = matches[0]
        definitions = [r['definition'] for path in (ROOT / 'content/items/definitions').glob('items-*.json')
                       for r in json.loads(path.read_bytes())['records']
                       if r['definition']['identity']['key'] == KEY]
        self.assertEqual(len(definitions), 1)
        definition = definitions[0]
        self.assertTrue(definition['materializable'])
        self.assertEqual(profile['semantics'], definition['semantics'])
        self.assertEqual(profile['authoring']['item'], definition['identity'])
        self.assertEqual(profile['production_binding']['target'], definition['identity'])
        self.assertEqual(profile['production_binding']['disposition'], 'EXACT')
        self.assertEqual(profile['admission'], {
            'materializable': True, 'stack_class': 'StackCapable',
            'legal_destinations': ['CharacterInventory', 'Ground'],
        })


if __name__ == '__main__':
    unittest.main()
