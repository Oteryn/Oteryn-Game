import copy
import unittest
import json
import subprocess
import sys
import tempfile
from pathlib import Path

import quest_v2_export as export


def definition(recipe=None):
    value = {
        'identity': {'key': 'oteryn:quest.example', 'revision': 'authored-r1'},
        'classification': 'OTERYN_AUTHORED_APPROXIMATION',
        'display_name': 'Example',
        'readiness': 'waiting_native_bindings',
    }
    if recipe is not None:
        value['definition_profile'] = 'oteryn_authored_v1'
        value['recipe'] = recipe
    return value


class ExportTests(unittest.TestCase):
    def project(self, recipe):
        return export.project(definition(recipe), 'canonical.json', '/records/0/definition', 'a' * 64)

    def test_unknowns_omitted(self):
        _, profile, _ = self.project({'requirements': {}, 'repeat': {'kind': 'unknown'}})
        self.assertEqual(set(profile['data']['profile']), {'fields'})

    def test_zero_and_false_are_explicit(self):
        _, profile, _ = self.project({'requirements': {'min_level': 0, 'premium': False}, 'repeat': {'kind': 'once'}})
        value = profile['data']['profile']
        self.assertEqual(value['required_level'], 0)
        self.assertIs(value['premium'], False)
        self.assertIs(value['repeatable'], False)

    def test_no_bool_as_number_or_overflow(self):
        for level in [True, -1, 65536, '20', None]:
            _, profile, _ = self.project({'requirements': {'min_level': level}})
            self.assertNotIn('required_level', profile['data']['profile'])

    def test_daily_has_source_cycle_only(self):
        _, profile, _ = self.project({'repeat': {'kind': 'daily', 'cooldown': 86400}})
        value = profile['data']['profile']
        self.assertTrue(value['repeatable'])
        field = next(f for f in value['fields'] if f['field_path'].endswith('chosen_repeat'))
        self.assertIn('86400', field['value']['value'])

    def test_reward_names_do_not_become_ids(self):
        _, profile, report = self.project({'requirements': {'prerequisites': ['Named Quest']}, 'reward_intents': [{'name': 'Sword', 'count': 1}]})
        value = profile['data']['profile']
        self.assertNotIn('reward_items', value)
        self.assertNotIn('prerequisites', value)
        self.assertTrue(any(f['field_path'].endswith('chosen_reward_intents') for f in value['fields']))
        self.assertEqual(len(report['omissions']), 7)

    def test_source_only_has_no_profile(self):
        declaration, profile, report = export.project(definition(), 'source.json', '/records/0/definition', 'a' * 64)
        self.assertIsNone(profile)
        self.assertEqual(declaration['identity'], definition()['identity'])
        self.assertFalse(report['profile_present'])

    def test_real_canonical_barbarian_hold(self):
        self.assertFalse(export.production_key('oteryn:quest.barbarian_test'))
        self.assertFalse(export.production_key('oteryn:source.quest.test_only'))
        self.assertFalse(export.production_key('oteryn:source.quest.evidence'))
        self.assertTrue(export.production_key('oteryn:source.quest.canonical_ref'))
        self.assertTrue(export.production_key('oteryn:quest.latest_adventure'))

    def test_unknown_nested_source_object_rejected(self):
        doc = self.document()
        doc['authoring_profiles'][0]['data']['profile']['donor_source_data'] = {}
        with self.assertRaises(ValueError):
            export.shape_check(doc)

    def test_candidate_object_not_text_rejected(self):
        doc = self.document()
        doc['records'][0]['fields'][0]['value']['value'] = {}
        with self.assertRaises(ValueError):
            export.shape_check(doc)

    def test_orphan_target_rejected(self):
        doc = self.document()
        doc['authoring_profiles'][0]['target']['revision'] = 'other-r1'
        with self.assertRaises(ValueError):
            export.shape_check(doc)

    def test_unsorted_fields_rejected(self):
        doc = self.document()
        doc['records'][0]['fields'].reverse()
        with self.assertRaises(ValueError):
            export.shape_check(doc)

    def test_check_is_deterministic_and_read_only(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            shards = root / "content/quests/definitions"
            shards.mkdir(parents=True)
            relative = "content/quests/definitions/quests.json"
            (shards / "index.json").write_text(json.dumps({"shards": [relative]}))
            (root / relative).write_text(json.dumps({"records": [{"definition": definition()}]}))
            command = [sys.executable, str(Path(export.__file__)), "--repo-root", str(root)]
            subprocess.run(command, check=True, capture_output=True)
            output = root / "tools/content-schema/quest-authoring/samples/v2-export/declarations.json"
            before = output.stat().st_mtime_ns
            subprocess.run(command + ["--check"], check=True, capture_output=True)
            self.assertEqual(before, output.stat().st_mtime_ns)
            output.write_text("stale")
            result = subprocess.run(command + ["--check"], capture_output=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(output.read_text(), "stale")

    def test_generated_keys_pass_production_marker_fence(self):
        for declaration in self.document()["records"]:
            for field in declaration["fields"]:
                self.assertTrue(export.production_key(field["field_path"]))

    def document(self):
        declaration, profile, _ = self.project({'requirements': {'min_level': 20, 'premium': True}, 'repeat': {'kind': 'once'}})
        doc = {'schema': 'OTERYN_WORLD_PROJECT_DECLARATIONS/v2', 'records': [declaration], 'authoring_profiles': [profile]}
        export.shape_check(doc)
        return doc

    def test_recipe_reference_distinguishes_parent_and_child_digests(self):
        recipe = {'requirements': {'min_level': 20}, 'stages': []}
        _, profile, _ = self.project(recipe)
        field = next(f for f in profile['data']['profile']['fields']
                     if f['field_path'].endswith('recipe_ref'))
        ref = json.loads(field['value']['value'])
        self.assertEqual(ref['definition_json_pointer'], '/records/0/definition')
        self.assertEqual(ref['json_pointer'], '/records/0/definition/recipe')
        self.assertEqual(ref['recipe_sha256'], export.sha(export.encode(recipe).encode()))
        self.assertEqual(ref['definition_sha256'],
                         export.sha(export.encode(definition(recipe)).encode()))

    def test_reserved_candidate_field_rejected(self):
        doc = self.document()
        doc['records'][0]['fields'][0]['field_path'] = 'oteryn:test.metadata'
        doc['records'][0]['fields'].sort(key=lambda f: f['field_path'])
        with self.assertRaisesRegex(ValueError, 'Candidate field shape'):
            export.shape_check(doc)


if __name__ == '__main__':
    unittest.main()
