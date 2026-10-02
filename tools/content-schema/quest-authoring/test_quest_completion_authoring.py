"""Corruption controls for chosen completion of existing SOURCE definitions."""
import copy
import unittest
from pathlib import Path
from unittest.mock import patch

import jsonschema
import quest_completion_authoring as tool


class QuestCompletionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root = Path(__file__).resolve().parents[3]
        index = tool.read(cls.root / 'content/quests/definitions/index.json')
        cls.source = []
        for path in index['shards']:
            for row in tool.read(cls.root / path)['records']:
                definition = row['definition']
                if definition.get('definition_profile') == 'oteryn_authored_v1':
                    continue
                definition.pop('oteryn_recipe', None)
                cls.source.append(row)
        cls.payload_path = cls.root / tool.DIRECTORY / 'recipes.json'
        cls.payload = tool.read(cls.payload_path)

    def mutated_payload(self, change):
        payload = copy.deepcopy(self.payload)
        change(payload)
        original = tool.read
        def replacement(path):
            return payload if path == self.payload_path else original(path)
        with patch.object(tool, 'read', side_effect=replacement):
            return tool.completion_records(self.root, self.source)

    def test_exact242_complete_without_duplicate_definitions_or_source_loss(self):
        completed = tool.completion_records(self.root, self.source)
        self.assertEqual(len(completed), 242)
        self.assertTrue(all(c['runtime_enabled'] is False for c in completed.values()))
        attached = tool.attach(self.root, self.source)
        for before, after in zip(self.source, attached):
            after['definition'].pop('oteryn_recipe', None)
            self.assertEqual(before, after)

    def test_source_owner_substitution_rejected(self):
        with self.assertRaisesRegex(ValueError, 'owner'):
            self.mutated_payload(lambda p: p['records'][0]['source_identity'].update(key='canary:quest/other'))

    def test_missing_selected_definition_rejected(self):
        with self.assertRaisesRegex(ValueError, 'ordered'):
            self.mutated_payload(lambda p: p['records'].pop())

    def test_foreign_wiki_revision_rejected(self):
        with self.assertRaisesRegex(ValueError, 'wiki evidence'):
            self.mutated_payload(lambda p: p['records'][0]['recipe']['source_refs'][0].update(revid=1))

    def test_family_scope_cannot_claim_another_quest(self):
        with self.assertRaisesRegex(ValueError, 'family coverage'):
            self.mutated_payload(lambda p: p['records'][0]['covered_wiki_titles'].append('Unrelated Quest'))

    def test_fake_source_reward_rejected(self):
        with self.assertRaises(jsonschema.ValidationError):
            self.mutated_payload(lambda p: p['records'][0]['recipe']['reward_intents'][0].update(basis='SOURCE_REFERENCE'))

    def test_disconnected_journey_rejected(self):
        with self.assertRaisesRegex(ValueError, 'disconnected'):
            self.mutated_payload(lambda p: p['records'][0]['recipe']['stages'][0].update(next=[]))

    def test_chosen_completion_cannot_claim_source_fields_ready(self):
        schema = tool.read(self.root / 'tools/content-schema/quest-authoring/quest_rollout.schema.json')
        rollout = tool.read(self.root / 'tools/content-schema/quest-authoring/samples/rollout/quest-rollout.json')
        record = next(r for r in rollout['records'] if r['chosen_source_recipe_complete'])
        record['definition_fields_ready'] = True
        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.Draft202012Validator(schema).validate(rollout)

    def test_chosen_overlay_cannot_hide_source_core_change(self):
        source = list(self.source)
        index = next(i for i, r in enumerate(source) if r['definition']['identity'] == self.payload['records'][0]['identity'])
        source[index] = copy.deepcopy(source[index])
        source[index]['definition']['requirements']['min_level'] = 999
        with self.assertRaisesRegex(ValueError, 'SOURCE core changed|missing or unrelated effective core changes|to_digest mismatch'):
            tool.completion_records(self.root, source)


if __name__ == '__main__':
    unittest.main()
