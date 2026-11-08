"""Every immutable cooking operand has native custody before its writer may charge it."""
import json
from pathlib import Path
import unittest


class CookingCustody(unittest.TestCase):
    def test_all_fifteen_recipe_inputs_and_outputs_are_materializable(self):
        root = Path(__file__).resolve().parents[2]
        recipes = json.loads((root/'content/quests/hot-cuisine-batch3.json').read_bytes())['recipes']
        profiles = {r['production_definition']['production_key']: r for r in
                    json.loads((root/'content/items/definitions/spell-native-profiles.json').read_bytes())['records']}
        for recipe in recipes:
            for operand in [*recipe['ingredients'], recipe['reward']]:
                with self.subTest(recipe=recipe['index'], item=operand['item']):
                    self.assertTrue(operand['item'] in profiles, operand['item'])
                    self.assertTrue(profiles[operand['item']]['admission']['materializable'])
        self.assertEqual(len(recipes), 15)


if __name__ == '__main__':
    unittest.main()
