"""Adversarial authoring admission checks for the selected 68-title release."""
import copy
import unittest
from pathlib import Path

import jsonschema
import authored_quest_authoring as tool


class AuthoredQuestTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root = Path(__file__).resolve().parents[3]
        cls.selection = tool.read(cls.root / tool.DIRECTORY / 'selection.json')
        cls.recipes = tool.read(cls.root / tool.DIRECTORY / 'recipes.json')['recipes']
        cls.specs = tool.read(cls.root / tool.SPECIFICATIONS)
        cls.schema = tool.read(cls.root / 'tools/content-schema/quest-authoring/quest_authored.schema.json')

    def validate(self, recipes):
        return tool.validate_recipes(self.selection, recipes, self.specs, self.schema)

    def changed(self):
        return copy.deepcopy(self.recipes)

    def test_all_selected_titles_are_individual_finite_recipes(self):
        self.validate(self.recipes)
        self.assertEqual(len(tool.build_records(self.root)), 68)
        self.assertEqual(len({r['summary'] for r in self.recipes}), 68)

    def test_missing_title_rejected(self):
        with self.assertRaises(ValueError):
            self.validate(self.recipes[:-1])

    def test_duplicate_title_rejected(self):
        recipes = self.changed(); recipes[-1] = recipes[0]
        with self.assertRaises(ValueError):
            self.validate(recipes)

    def test_other_wiki_revision_cannot_substitute(self):
        recipes = self.changed(); recipes[0]['source_refs'][0]['revid'] += 1
        with self.assertRaisesRegex(ValueError, 'provenance'):
            self.validate(recipes)

    def test_cyclic_journey_rejected(self):
        recipes = self.changed(); recipes[0]['stages'][-1]['next'] = ['s1']
        with self.assertRaisesRegex(ValueError, 'cyclic'):
            self.validate(recipes)

    def test_unreachable_objective_rejected(self):
        recipes = self.changed(); recipes[0]['stages'][0]['next'] = []
        with self.assertRaisesRegex(ValueError, 'unreachable'):
            self.validate(recipes)

    def test_unknown_target_binding_cannot_be_smuggled_as_native_id(self):
        recipes = self.changed(); recipes[0]['stages'][0]['native_target_id'] = 123
        with self.assertRaises(jsonschema.ValidationError):
            self.validate(recipes)

    def test_chosen_reward_cannot_be_promoted_to_source_fact(self):
        recipes = self.changed(); recipes[0]['reward_intents'][0]['basis'] = 'SOURCE_REFERENCE'
        with self.assertRaises(jsonschema.ValidationError):
            self.validate(recipes)

    def test_runtime_enable_is_rejected_by_definition_schema(self):
        definition = tool.build_records(self.root)[0]['definition']
        definition['runtime_enabled'] = True
        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.Draft202012Validator(self.schema).validate(definition)

    def test_duplicate_objective_shell_rejected(self):
        recipes = self.changed()
        recipes[0]['stages'][1]['objective'] = recipes[0]['stages'][0]['objective']
        with self.assertRaisesRegex(ValueError, 'shell'):
            self.validate(recipes)

    def test_authored_rollout_cannot_hide_its_approximation(self):
        payload = tool.read(self.root / 'tools/content-schema/quest-authoring/samples/rollout/quest-rollout.json')
        schema = tool.read(self.root / 'tools/content-schema/quest-authoring/quest_rollout.schema.json')
        row = next(r for r in payload['records'] if r['binding_scope'] == 'authored')
        row['approximation_applied'] = False
        row['approximation_evidence'] = []
        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.Draft202012Validator(schema).validate(payload)


if __name__ == '__main__':
    unittest.main()
