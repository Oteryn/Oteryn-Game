"""Exact, conservative quest Item reference enrichment."""
import unittest
import json
from pathlib import Path
import copy
from quest_item_keys import catalogue, enrich, resolve, validate_references, extend_completion_schema


def definition(key, name):
    return {'identity': {'family': 'Item', 'key': key, 'revision': 'r1'},
            'semantics': {'presentation': {'state': 'KNOWN', 'value': {
                'name': {'state': 'KNOWN', 'value': name}}}}}


class QuestItemKeysTests(unittest.TestCase):
    def setUp(self):
        self.definitions = [definition('oteryn:item.pearl', 'White Pearl'),
                            definition('oteryn:item.key1', 'Key'),
                            definition('oteryn:item.key2', 'KEY')]
        self.index = catalogue(self.definitions)

    def test_unique_and_case_insensitive_exact_only(self):
        self.assertEqual(resolve('WHITE pearl', self.index)['item'],
                         self.definitions[0]['identity'])
        for name in ('Pearl', ' White Pearl', 'White Pearls', 'missing'):
            self.assertEqual(resolve(name, self.index),
                             {'name': name, 'item_resolution': 'unresolved'})

    def test_ambiguous_and_non_item(self):
        self.assertEqual(resolve('key', self.index)['item_resolution'], 'ambiguous')
        self.assertNotIn('item', resolve('key', self.index))
        self.assertEqual(resolve('dead trees', self.index)['item_resolution'], 'not_an_item')

    def test_scope_and_report(self):
        recipe = {'stages': [{'kind': k, 'targets': ['White Pearl', 'Key', 'missing', 'dead trees']}
                             for k in ('collect', 'use', 'kill', 'talk', 'explore')],
                  'reward_intents': [{'kind': 'item', 'name': 'White Pearl'},
                                     {'kind': 'outfit', 'name': 'White Pearl'}]}
        report = enrich(recipe, self.index)
        self.assertEqual(report['counts'], {'resolved': 3, 'ambiguous': 2,
                                          'unresolved': 2, 'not_an_item': 2})
        self.assertEqual(report['names'][0]['candidate_keys'], ['oteryn:item.key1', 'oteryn:item.key2'])
        for stage in recipe['stages'][2:]:
            self.assertNotIn('target_items', stage)
        self.assertNotIn('item', recipe['reward_intents'][1])
        self.assertEqual(enrich(recipe, self.index), report)
        validate_references(recipe, self.definitions)
        recipe['reward_intents'][0]['item']['key'] = 'oteryn:item.missing'
        with self.assertRaises(ValueError):
            validate_references(recipe, self.definitions)

    def test_report_covers_resolution_names_in_all_quest_files(self):
        root = Path(__file__).resolve().parents[3] / 'content/quests'
        report = json.loads((root / 'item-key-resolution.json').read_text())
        reported = {row['name'] for row in report['names']}
        encountered = set()

        def walk(value):
            if isinstance(value, dict):
                if 'item_resolution' in value:
                    encountered.add(value['name'])
                for child in value.values():
                    walk(child)
            elif isinstance(value, list):
                for child in value:
                    walk(child)

        for path in root.rglob('*.json'):
            if path.name != 'item-key-resolution.json':
                walk(json.loads(path.read_text()))
        self.assertTrue(encountered)
        self.assertEqual(encountered - reported, set())

    def test_terminal_normalization_preserves_verified_enrichment(self):
        import quest_recipe_refinements
        import quest_terminal_stage_refinements
        root = Path(__file__).resolve().parents[3]
        payload = json.loads((root / 'tools/content-schema/quest-authoring/samples/completion242/recipes.json').read_text())
        rows, _ = quest_recipe_refinements.apply_refinements(root, payload)
        row = next(row for row in rows if row['identity']['key'] == 'oteryn:quest.bear_room_quest')
        original, _ = quest_terminal_stage_refinements.normalize_payload(root, row)
        from quest_item_keys import item_definitions
        index = catalogue(item_definitions(root))
        enrich(row['recipe'], index)
        normalized, _ = quest_terminal_stage_refinements.normalize_payload(root, row)
        enrich(original['recipe'], index)
        self.assertEqual(normalized, original)
        row['recipe']['reward_intents'][0]['item_resolution'] = 'not_an_item'
        with self.assertRaisesRegex(ValueError, 'Item enrichment differs'):
            quest_terminal_stage_refinements.normalize_payload(root, row)

    def test_schema_loads_old_and_enriched_recipes_and_rejects_bad_reference(self):
        import jsonschema
        here = Path(__file__).parent
        original = json.loads((here / 'quest_completion.schema.json').read_text())
        schema = extend_completion_schema(here, original)['properties']['recipe']
        recipes = json.loads((here / 'samples/completion242/recipes.json').read_text())['records']
        recipe = copy.deepcopy(next(row['recipe'] for row in recipes
                                    if any(r['kind'] == 'item' for r in row['recipe']['reward_intents'])))
        jsonschema.validate(recipe, schema)
        enrich(recipe, self.index)
        jsonschema.validate(recipe, schema)
        reward = next(row for row in recipe['reward_intents'] if row['kind'] == 'item')
        reward['item'] = {'family': 'Item', 'key': 'bad', 'revision': 'r1'}
        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.validate(recipe, schema)


if __name__ == '__main__':
    unittest.main()
