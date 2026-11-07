#!/usr/bin/env python3
import copy
import unittest
from build_canonical_monster_profiles import union_profiles, extend_illusion_appearances
import build_canonical_monster_profiles as exporter
from pathlib import Path
from unittest.mock import patch


def ref(family, key):
    return {'family': family, 'key': key, 'revision': 'definition-r1'}


def fixture():
    creature = ref('Creature', 'oteryn:creature.rat')
    presentation = ref('Presentation', 'oteryn:presentation.rat')
    behavior = ref('Behavior', 'oteryn:behavior.rat')
    active = {'schema': 'OTERYN_NATIVE_CREATURE_PROFILES/v1', 'records': [
        {'profile': {'target': creature, 'data': {'profile': {'details': {'display_name': 'Rat'}, 'preserved': True}}}, 'presentation': presentation}]}
    presentation_profile = {'target': presentation, 'data': {'kind': 'Presentation'}}
    presentations = {'schema': 'OTERYN_NATIVE_PRESENTATION_PROFILES/v1', 'records': [copy.deepcopy(presentation_profile)]}
    rows = [{'definition': {'identity': creature, 'presentation': presentation, 'behavior': behavior},
             'authoring': {'profile': {'details': {'display_name': 'Rat'}, 'changed': True}}}]
    declarations = [presentation_profile, {'target': behavior, 'data': {'kind': 'Behavior'}}]
    return active, presentations, rows, declarations


class CanonicalUnion(unittest.TestCase):
    def test_existing_profile_wins_without_rewriting_inputs(self):
        args = fixture()
        original = copy.deepcopy(args)
        creatures, presentations, appended = union_profiles(*args)
        self.assertEqual(creatures, args[0])
        self.assertEqual(presentations, args[1])
        self.assertEqual(appended, [])
        self.assertEqual(args, original)

    def test_new_exact_identity_uses_actual_authoring_and_presentation(self):
        active, presentations, rows, declarations = fixture()
        active['records'] = []
        presentations['records'] = []
        creatures, presentations, appended = union_profiles(active, presentations, rows, declarations)
        self.assertEqual(creatures['records'][0]['profile']['data'], rows[0]['authoring'])
        self.assertEqual(creatures['records'][0]['behavior'], declarations[1])
        self.assertEqual(presentations['records'], [declarations[0]])
        self.assertEqual(appended, [rows[0]['definition']['identity']])

    def test_conflicting_presentation_and_missing_exact_profile_refuse(self):
        active, presentations, rows, declarations = fixture()
        active['records'] = []
        presentations['records'][0]['data'] = {'conflicting': True}
        with self.assertRaises(ValueError):
            union_profiles(active, presentations, rows, declarations)
        presentations['records'] = []
        with self.assertRaises(KeyError):
            union_profiles(active, presentations, rows, declarations[:1])

    def test_native_display_name_alias_is_excluded_without_renaming(self):
        active, presentations, rows, declarations = fixture()
        rows = copy.deepcopy(rows)
        rows[0]['definition']['identity']['key'] = 'oteryn:creature.another_rat'
        rows[0]['authoring']['profile']['details']['display_name'] = 'rAT'
        creatures, result_presentations, appended = union_profiles(active, presentations, rows, declarations)
        self.assertEqual(creatures, active)
        self.assertEqual(result_presentations, presentations)
        self.assertEqual(appended, [])
        self.assertEqual(rows[0]['authoring']['profile']['details']['display_name'], 'rAT')

    def test_duplicate_active_keys_refuse_even_across_revision(self):
        args = fixture()
        duplicate = copy.deepcopy(args[0]['records'][0])
        duplicate['profile']['target']['revision'] = 'another-revision'
        args[0]['records'].append(duplicate)
        with self.assertRaises(ValueError):
            union_profiles(*args)


class IllusionAppearanceClosure(unittest.TestCase):
    def inputs(self):
        active, presentations, rows, _ = fixture()
        target = active['records'][0]['profile']['target']
        active['records'][0]['profile']['data'] = {'profile': {'details': {'display_name': 'Rat', 'flags': {'illusionable': True}}}}
        presentations['records'][0]['data'] = {'profile': {'asset_binding': 'canary.appearance:outfit/100'}}
        rows[0]['source_bindings'] = [{'disposition': 'EXACT', 'target': target,
            'source_key': 'oteryn:source.canary', 'identity_namespace': 'canary/monster-file',
            'external_id': 'mammals/rat'}]
        spell_appearances = {'schema': 'OTERYN_NATIVE_SPELL_APPEARANCES/v1',
            'default_source_sha256': exporter.appearance.DEFAULT_SHA, 'records': []}
        qualified = {'creature': target, 'look_type': 100, 'source': {'sha256': 'a' * 64},
                     'qualification_sha256': 'b' * 64}
        return active, presentations, rows, spell_appearances, qualified

    def test_missing_link_uses_exact_source_and_preserves_illusion_flag(self):
        active, presentations, rows, previous, qualified = self.inputs()
        original = copy.deepcopy((active, previous))
        with patch.object(exporter.appearance, 'git', return_value=exporter.appearance.PIN.encode()), \
             patch.object(exporter.appearance, 'pinned', return_value=(b'', '', exporter.appearance.DEFAULT_SHA)), \
             patch.object(exporter.appearance, 'record', return_value=qualified) as source_owner:
            result, added = extend_illusion_appearances(active, presentations, rows, previous, Path('/source'))
        source_owner.assert_called_once_with(Path('/source'), 'data-otservbr-global/monster/mammals/rat.lua',
                                             active['records'][0]['profile']['target'])
        self.assertEqual(result['records'], [qualified])
        self.assertEqual(len(added), 1)
        self.assertEqual((active, previous), original)
        self.assertTrue(active['records'][0]['profile']['data']['profile']['details']['flags']['illusionable'])

    def test_source_outfit_drift_refuses_without_rewriting_canonical_policy(self):
        active, presentations, rows, previous, qualified = self.inputs()
        qualified['look_type'] = 101
        with patch.object(exporter.appearance, 'git', return_value=exporter.appearance.PIN.encode()), \
             patch.object(exporter.appearance, 'pinned', return_value=(b'', '', exporter.appearance.DEFAULT_SHA)), \
             patch.object(exporter.appearance, 'record', return_value=qualified):
            with self.assertRaisesRegex(ValueError, 'outfit differs'):
                extend_illusion_appearances(active, presentations, rows, previous, Path('/source'))
        self.assertEqual(presentations['records'][0]['data']['profile']['asset_binding'], 'canary.appearance:outfit/100')
        self.assertEqual(previous['records'], [])

    def test_already_qualified_link_keeps_original_row_and_does_not_reissue(self):
        active, presentations, rows, previous, qualified = self.inputs()
        previous['records'] = [qualified]
        with patch.object(exporter.appearance, 'git', return_value=exporter.appearance.PIN.encode()), \
             patch.object(exporter.appearance, 'pinned', return_value=(b'', '', exporter.appearance.DEFAULT_SHA)), \
             patch.object(exporter.appearance, 'record') as source_owner:
            result, added = extend_illusion_appearances(active, presentations, rows, previous, Path('/source'))
        source_owner.assert_not_called()
        self.assertEqual(result, previous)
        self.assertEqual(added, [])


if __name__ == '__main__':
    unittest.main()
