"""Presentation repair must preserve the native reader's closed execution binding."""
import copy
import json
from pathlib import Path
import unittest

from complete_player_sounds import synchronize_native_profiles

ROOT = Path(__file__).parent / 'samples'


class NativePresentationSyncTests(unittest.TestCase):
    def setUp(self):
        self.catalog = json.loads((ROOT / 'executable-spell-catalog.json').read_bytes())
        self.profiles = ROOT / 'native-spell-profiles.json'
        identity = json.loads(self.profiles.read_bytes())['profiles'][0]['spell']['identity']
        self.row = next(row for row in self.catalog['bundles']
                        if row['bundle']['spell']['identity'] == identity)

    def test_current_catalog_matches_all_closed_native_headers_and_dependencies(self):
        result, changes = synchronize_native_profiles(self.catalog, self.profiles)
        self.assertEqual(changes, [])
        self.assertEqual(result, json.loads(self.profiles.read_bytes()))

    def test_presentation_only_update_retains_closed_execution(self):
        self.row['bundle']['spell']['presentation'] = {'cast_cue': 'canary.sound:spell_or_rune'}
        result, _ = synchronize_native_profiles(self.catalog, self.profiles)
        profile = next(row for row in result['profiles']
                       if row['spell']['identity'] == self.row['bundle']['spell']['identity'])
        self.assertEqual(profile['spell'], self.row['bundle']['spell'])
        self.assertEqual(profile['dependencies'], self.row['dependencies'])

    def test_cost_parameter_or_dependency_change_is_refused(self):
        for mutate in [lambda row: row['bundle']['spell']['costs'].update(mana=1),
                       lambda row: row['bundle']['spell']['execution'].update(native_behavior={}),
                       lambda row: row['dependencies']['effects'].append({'operation': 'substitution'})]:
            with self.subTest(mutate=mutate):
                original = copy.deepcopy(self.row)
                mutate(self.row)
                with self.assertRaisesRegex(ValueError, 'presentation only'):
                    synchronize_native_profiles(self.catalog, self.profiles)
                self.row.clear()
                self.row.update(original)

    def test_duplicate_identity_missing_profile_and_unqualified_revision_refused(self):
        for mutate in [lambda doc: doc['bundles'].__setitem__(0, copy.deepcopy(self.row)),
                       lambda doc: doc.update(revision='unqualified'),
                       lambda doc: doc['bundles'].remove(self.row)]:
            with self.subTest(mutate=mutate):
                document = copy.deepcopy(self.catalog)
                mutate(document)
                with self.assertRaises(ValueError):
                    synchronize_native_profiles(document, self.profiles)


if __name__ == '__main__':
    unittest.main()
