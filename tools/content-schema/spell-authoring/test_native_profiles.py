"""Closed-coverage and qualification regressions for generated native profiles."""
import copy
import json
from pathlib import Path
import tempfile
import unittest

from jsonschema import ValidationError

import build_native_profiles as builder
from native_house_movement import BEHAVIOURS
from verify_formal_schema import LIGHT_HEALING


class NativeProfileTests(unittest.TestCase):
    def profiles(self):
        path = Path(__file__).parent / 'samples/native-spell-profiles.json'
        return json.loads(path.read_text())['profiles']

    def bundle(self, directory):
        spell = copy.deepcopy(LIGHT_HEALING)
        spell['spell'].update(name='House Guest List', execution={
            'native_behavior': copy.deepcopy(BEHAVIOURS['house guest list'])})
        spell['spell']['identity'].update(key='candidate:spell/house_guest_list', revision=builder.REVISION)
        docs = {'spell': spell, 'dependencies': {'abilities': [], 'effects': [], 'formulas': []},
                'catalog': {'definitions': []},
                'manifest': {'sources': [{'repository': 'opentibiabr/canary',
                                         'revision': '99902524e052f37574194466c2949c576e4ab269'}],
                             'entries': [{'source_index': 0, 'source_file': 'data/scripts/spells/house/house_guest_list.lua',
                                          'source_line': 1, 'source_field': 'onCastSpell', 'kind': 'script',
                                          'status': 'resolved_native_behavior',
                                          'destination': '/spell/spell/execution/native_behavior',
                                          'resolution': 'Synthetic adapter schema regression fixture.'}]}}
        self.write(directory, docs)
        return docs

    def write(self, directory, documents):
        directory.mkdir(exist_ok=True)
        for name, document in documents.items():
            (directory / (name + '.json')).write_text(json.dumps(document))

    def test_exact_identity_coverage_and_deterministic_sort(self):
        profiles = self.profiles()
        document = builder.assemble(list(reversed(profiles)))
        self.assertEqual(67, len(document['profiles']))
        self.assertEqual(2, len(builder.BARRIER_IDS))
        self.assertEqual(builder.REVISION, document['revision'])
        self.assertEqual(sorted((p['carrier'], p['name']) for p in profiles),
                         [(p['carrier'], p['name']) for p in document['profiles']])

    def test_duplicate_and_missing_records_fail(self):
        profiles = self.profiles()
        with self.assertRaisesRegex(ValueError, 'duplicate native profile'):
            builder.assemble(profiles + [copy.deepcopy(profiles[0])])
        with self.assertRaisesRegex(ValueError, 'coverage mismatch'):
            builder.assemble(profiles[:-1])
        # Equal cardinality is insufficient when a different identity is inserted.
        profiles[0]['name'] = profiles[0]['spell']['name'] = 'not a covered spell'
        with self.assertRaisesRegex(ValueError, 'coverage mismatch'):
            builder.assemble(profiles)

    def test_valid_foreign_native_recipe_cannot_replace_another_spell_family(self):
        profiles = self.profiles()
        avatar = next(profile for profile in profiles if profile['name'] == 'Avatar of Balance')
        house = next(profile for profile in profiles if profile['name'] == 'House Guest List')
        avatar['execution'] = copy.deepcopy(house['execution'])
        avatar['spell']['execution'] = copy.deepcopy(house['execution'])
        with self.assertRaisesRegex(ValueError, 'native operation owner must be avatar_state'):
            builder.assemble(profiles)

    def test_barrier_item_bindings_are_owned_by_the_exact_rune(self):
        profiles = self.profiles()
        wall = next(profile for profile in profiles if profile['name'] == 'Magic Wall Rune')
        growth = next(profile for profile in profiles if profile['name'] == 'Wild Growth Rune')
        wall['dependencies'] = copy.deepcopy(growth['dependencies'])
        with self.assertRaisesRegex(ValueError, 'wrong barrier created_item binding'):
            builder.assemble(profiles)

    def test_aliased_execution_and_headers_must_match(self):
        for field in ('name', 'carrier', 'execution'):
            profiles = self.profiles()
            profiles[0][field] = 'tampered'
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, 'disagrees with spell'):
                builder.assemble(profiles)

    def test_tampered_native_parameters_even_with_matching_execution_alias_fail(self):
        profiles = self.profiles()
        profiles[0]['execution']['native_behavior']['parameters']['lua_body'] = 'return true'
        profiles[0]['spell']['execution'] = copy.deepcopy(profiles[0]['execution'])
        with self.assertRaises(ValidationError):
            builder.assemble(profiles)

    def test_stale_identity_or_reference_revisions_fail(self):
        for field in ('identity', 'reference'):
            profiles = self.profiles()
            if field == 'identity':
                profiles[0]['spell']['identity']['revision'] = 'spell-p2-r18'
            else:
                profiles[0]['dependencies']['effects'].append({
                    'created_item': {'family': 'Item', 'key': 'candidate:item/40450', 'revision': 'spell-p2-r18'}})
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, 'expected exact ' + (builder.REVISION if field == 'identity' else builder.ITEM_REVISION)):
                builder.assemble(profiles)

    def test_valid_bundle_schema_then_unknown_or_tampered_native_parameters(self):
        with tempfile.TemporaryDirectory() as root:
            directory = Path(root) / 'instant-house_guest_list'
            documents = self.bundle(directory)
            self.assertEqual('House Guest List', builder.profile_from_bundle(directory)['name'])
            native = documents['spell']['spell']['execution']['native_behavior']
            native['key'] = 'arbitrary_lua_executor'
            self.write(directory, documents)
            with self.assertRaisesRegex(ValueError, 'unknown native behavior'):
                builder.profile_from_bundle(directory)
            native['key'] = 'house_access'
            native['parameters']['lua_body'] = 'return true'
            self.write(directory, documents)
            with self.assertRaises(ValueError):
                builder.profile_from_bundle(directory)

    def test_unresolved_refs_and_manifest_uncertainty_are_rejected(self):
        with tempfile.TemporaryDirectory() as root:
            directory = Path(root) / 'instant-house_guest_list'
            documents = self.bundle(directory)
            documents['dependencies']['abilities'] = [{
                'identity': {'key': 'candidate:ability/test', 'revision': builder.REVISION},
                'kind': 'spell', 'needs_target': False, 'needs_direction': False,
                'effects': [{'family': 'Effect', 'key': 'candidate:effect/missing', 'revision': builder.REVISION}]}]
            self.write(directory, documents)
            with self.assertRaises(ValueError):
                builder.profile_from_bundle(directory)
            documents = self.bundle(directory)
            entry = documents['manifest']['entries'][0]
            entry['status'] = 'unresolved_semantics'
            self.write(directory, documents)
            with self.assertRaisesRegex(ValueError, 'unresolved_semantics'):
                builder.profile_from_bundle(directory)


if __name__ == '__main__':
    unittest.main()
