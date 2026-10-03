import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import merge_completion_packets as merger


class CompletionPacketMergeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.baseline = self.root / 'baseline'
        self.baseline.mkdir()
        merger.write(self.baseline / 'population-index.json', {'monsters': []})
        merger.write(self.baseline / 'completion-quality.json', {'accepted': True})
        (self.baseline / 'encounters').mkdir()

    def packet(self, name, rows):
        folder = self.root / name
        merger.write(folder / 'completion.json', {
            'baseline_index_sha256': merger.sha(self.baseline / 'population-index.json'),
            'index_monsters': rows})
        return folder

    def test_cross_baseline_packet_fails_before_creating_output(self):
        packet = self.packet('wrong', [])
        merger.write(packet / 'completion.json', {
            'baseline_index_sha256': 'different', 'index_monsters': []})
        output = self.root / 'out'
        with self.assertRaisesRegex(ValueError, 'another baseline'):
            merger.merge(self.baseline, [packet], output)
        self.assertFalse(output.exists())

    def test_overlapping_writers_fail_before_installing_any_bundle(self):
        packets = [self.packet(name, [{'monster': 'rat'}]) for name in ('a', 'b')]
        with self.assertRaisesRegex(ValueError, 'conflicting completion writers'):
            merger.merge(self.baseline, packets, self.root / 'out')

    def test_symlink_input_is_rejected(self):
        source = self.root / 'links'
        source.mkdir()
        (source / 'escape').symlink_to(self.baseline / 'population-index.json')
        with self.assertRaisesRegex(ValueError, 'symlink'):
            merger.immutable_copy(source, self.root / 'copy')

    def test_completion_cannot_erase_estimate_qualification(self):
        merger.write(self.baseline / 'population-index.json', {'monsters': [{
            'monster': 'rat', 'completion_flags': ['OWNER_ACCEPTED_NON_GLOBAL_ESTIMATE']}]})
        packet = self.packet('estimate', [{'monster': 'rat',
                             'resolved_completion_flags': ['OWNER_ACCEPTED_NON_GLOBAL_ESTIMATE']}])
        with self.assertRaisesRegex(ValueError, 'erase estimate'):
            merger.merge(self.baseline, [packet], self.root / 'out')

    def test_only_declared_resolved_flags_are_removed(self):
        merger.write(self.baseline / 'population-index.json', {'monsters': [{
            'monster': 'rat', 'completion_flags': ['PARTIAL', 'UNVERIFIED']}]})
        packet = self.packet('restored', [{'monster': 'rat', 'resolved_completion_flags': ['PARTIAL'],
                                         'completion_flags': ['CORE_RESTORED']}])
        with patch.object(merger.population, 'merge_population', return_value={'monsters': []}) as merge:
            merger.merge(self.baseline, [packet], self.root / 'out')
        self.assertEqual(['CORE_RESTORED', 'UNVERIFIED'], merge.call_args.args[3][0][1]['completion_flags'])

    def test_unchanged_encounter_bytes_are_preserved(self):
        encounter = self.baseline / 'encounters' / 'quest' / 'encounter.json'
        merger.write(encounter, {'identity': 'quest', 'rules': []})
        packet = self.packet('a', [])
        output = self.root / 'out'
        with patch.object(merger.population, 'merge_population', return_value={'monsters': []}):
            result = merger.merge(self.baseline, [packet], output)
        self.assertEqual(0, result['added'])
        self.assertEqual(encounter.read_bytes(), (output / 'encounters/quest/encounter.json').read_bytes())
        self.assertTrue(json.loads((output / 'completion-quality.json').read_text())['baseline_quality']['accepted'])

    def test_derived_types_cannot_reuse_parent_source_identity(self):
        def binding(key):
            return {'source_key': 'canary', 'source_revision': 'pin',
                    'identity_namespace': 'monster-file', 'external_id': 'parent.lua',
                    'target': {'family': 'Creature', 'key': key, 'revision': 'r1'}}
        with self.assertRaisesRegex(ValueError, 'duplicate source identity'):
            merger.validate_final_bindings({'source_identity_bindings': [binding('parent'), binding('derived')]})

    def test_registered_type_fragments_are_distinct_and_fully_admitted(self):
        def binding(key):
            return {'source_key': 'canary', 'source_revision': 'pin',
                    'identity_namespace': 'monster-file', 'external_id': 'parent.lua#type=' + key,
                    'target': {'family': 'Creature', 'key': key, 'revision': 'r1'}}
        bindings = [binding(key) for key in ('parent', 'derived')]
        stage = {'source_identity_bindings': bindings, 'declarations': [],
                 'authoring_profiles': [{'target': row['target'], 'data': {'kind': 'Creature'}} for row in bindings]}
        self.assertEqual(2, merger.validate_final_bindings(stage)['unique_source_bindings'])
        stage['authoring_profiles'].pop()
        with self.assertRaisesRegex(ValueError, 'cover exactly'):
            merger.validate_final_bindings(stage)


if __name__ == '__main__':
    unittest.main()
