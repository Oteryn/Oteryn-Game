"""Partial admission must retain source gaps and every encoded rule."""
import copy
import json
import unittest
from pathlib import Path

import prepare_encounter_completion as prep


class PartialEncounterTests(unittest.TestCase):
    def load(self, name):
        directory = prep.ROOT / 'samples' / name
        return (json.loads((directory / 'encounter.json').read_text()),
                json.loads((directory / 'manifest.json').read_text()))

    def test_partial_successors_preserve_all_rules_and_original_source(self):
        for name in prep.OMISSIONS:
            encounter, manifest = self.load(name)
            frozen = copy.deepcopy(manifest)
            successor, next_manifest, omitted = prep.partial_successor(encounter, manifest, name)
            self.assertEqual(encounter, successor)
            self.assertEqual(frozen, manifest)
            self.assertEqual(1, len(omitted))
            self.assertEqual('unresolved_semantics', omitted[0]['status'])
            self.assertEqual(manifest['sources'], next_manifest['sources'])
            self.assertFalse(any(r['status'] == 'unresolved_semantics' for r in next_manifest['entries']))
            self.assertIn(prep.OMISSIONS[name], next_manifest['entries'][manifest['entries'].index(omitted[0])]['resolution'])

    def test_unknown_unresolved_mechanic_cannot_be_dropped(self):
        encounter, manifest = self.load('ferumbras_mortal_shell')
        with self.assertRaisesRegex(ValueError, 'unrecognized'):
            prep.partial_successor(encounter, manifest, 'unknown_boss')

    def test_encoded_mechanic_cannot_be_omitted_by_manifest_switch(self):
        encounter, manifest = self.load('ferumbras_mortal_shell')
        row = next(r for r in manifest['entries'] if r['status'] == 'unresolved_semantics')
        row['destination'] = '/encounter/rules/0'
        with self.assertRaisesRegex(ValueError, 'encoded'):
            prep.partial_successor(encounter, manifest, 'ferumbras_mortal_shell')

    def test_missing_gap_is_not_false_partial_approval(self):
        encounter, manifest = self.load('ferumbras_mortal_shell')
        manifest['entries'] = [r for r in manifest['entries'] if r['status'] != 'unresolved_semantics']
        with self.assertRaisesRegex(ValueError, 'exactly one'):
            prep.partial_successor(encounter, manifest, 'ferumbras_mortal_shell')

    def test_actor_omission_does_not_approve_missing_stats_or_partial_source(self):
        entries = [
            {'kind': 'field', 'source_field': 'health', 'status': 'unresolved_semantics'},
            {'kind': 'script', 'source_field': 'top-level script before mType:register',
             'status': 'unresolved_semantics'},
            {'kind': 'field', 'source_field': 'attacks[2]', 'status': 'unresolved_semantics',
             'resolution': 'registered instant spell "foo" has custom logic', 'destination': '/bad'},
            {'kind': 'script', 'source_field': 'mType.onThink', 'status': 'unresolved_semantics'},
        ]
        manifest = {'entries': entries}
        omitted = prep.approved_actor_omissions(manifest)
        self.assertEqual(2, len(omitted))
        self.assertEqual('unresolved_semantics', entries[0]['status'])
        self.assertEqual('unresolved_semantics', entries[1]['status'])
        self.assertEqual('approved_omission', entries[2]['status'])
        self.assertNotIn('destination', entries[2])
        self.assertEqual('/bad', omitted[0]['destination'])

    def test_completion_receipt_contains_recursive_and_unresolved_encounter_actors(self):
        receipt = json.loads((prep.ROOT / 'samples' / 'encounter-completion-20261002.json').read_text())
        actors = {row['monster']: row for row in receipt['actors']}
        self.assertEqual(31, len(actors))
        self.assertTrue({'cloak_of_terror', 'ferumbras_mortal_shell', 'walker'} <= set(actors))
        self.assertTrue(all(row['schema_valid'] and not row['validation_errors'] for row in actors.values()))
        professor = actors['professor_maxxen']
        omitted = professor['failed_summon_omission']
        self.assertEqual('canary:creature/glooth_smasher', omitted['original_action']['creature']['key'])
        self.assertFalse(omitted['alias_created'])
        self.assertEqual('zimbadev/crystalserver', omitted['corroborating_source']['repository'])
        self.assertIn('SOURCE_UNREGISTERED_SUMMON_OMITTED', professor['flags'])
        self.assertEqual('SOURCE_DONOR_PARTIAL', professor['classification'])
        self.assertFalse(receipt['native_reference_closure_verified'])
        self.assertFalse(receipt['runtime_qualified'])


if __name__ == '__main__':
    unittest.main()
