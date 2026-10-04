"""R56 data completeness, source identity and fail-closed closure tests."""
import copy
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import project_equipment_candidates as project

ROOT = Path(__file__).resolve().parents[3]
SOURCES = Path('/workspace/spell-sources')


class EquipmentProjectionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.values, cls.headers = project.build(ROOT, SOURCES)
        cls.audit = cls.values['lane-audit.json']

    def test_complete_population_and_source_headers(self):
        self.assertEqual(len(self.audit['records']), 25)
        self.assertEqual(len(self.headers), 25)
        for row in self.audit['records']:
            raw = self.headers[row['source_header_path']]
            archived = ROOT / 'imports/spells/r28/player-source-bundles' / row['source_header_path']
            self.assertEqual(raw, archived.read_bytes())
            self.assertEqual(project.sha(raw), row['source_header_sha256'])

    def test_two_complete_data_candidates_are_not_runtime_admitted(self):
        full = [row for row in self.audit['records'] if row['full_candidate']]
        self.assertEqual({row['name'].casefold() for row in full}, {'focus harmony', 'focus serenity'})
        self.assertTrue(all(row['registration_key'].startswith('canary-main-current/') for row in full))
        for row in full:
            self.assertEqual(row['status'], 'CANDIDATE_SCHEMA_VALID')
            self.assertTrue(row['native_data_model_complete'])
            self.assertFalse(row['source_alias_to_existing_native_profile'])
            self.assertFalse(row['reader_acceptance_qualified'])
            self.assertTrue(row['runtime_capability_blocked'])
            self.assertFalse(row['exact_native_profile_match'])
            self.assertEqual(row['required_operations_unrepresented'], [])
            self.assertEqual(len(row['scoped_helper_proofs']), 9)
            self.assertTrue(row['source_costs_preserved'])

    def test_no_fake_fixed_physical_projection_or_cost_alias(self):
        partial = self.values['partial-data.json']
        self.assertEqual(len(partial['partial_targets']), 14)
        self.assertEqual(len(partial['proposed_native_bindings']), 16)
        for binding in partial['proposed_native_bindings']:
            self.assertFalse(binding['source_receipt_promoted'])
            self.assertFalse(binding['source_binding_qualified'])
        for row in self.audit['records']:
            self.assertFalse(row['native_execution_qualified'])
            self.assertFalse(row['runtime_activation'])
            if row['status'] == 'BLOCKED':
                self.assertNotIn(row['source_header_path'].replace('source-header', 'spell'), self.values)

    def test_crystal_virtue_assignment_not_falsely_toggled(self):
        rows = [row for row in self.audit['records'] if row['registration_key'].startswith('crystal-')
                and row['name'].casefold().startswith('virtue of')]
        self.assertEqual(len(rows), 3)
        for row in rows:
            self.assertEqual(row['proposed_source_parameter_patch'], {'toggle_same_stance_off': False})
            self.assertFalse(row['full_candidate'])
            self.assertTrue(row['source_parameter_differences'])

    def test_scoped_helper_drift_fails_closed(self):
        actual = project.base.source_file
        def altered(root, revision, path):
            raw = actual(root, revision, path)
            if revision == project.PINS['canary'] and path.endswith('player.cpp'):
                raw = raw.replace(b'buildHarmony(5);', b'buildHarmony(4);')
            return raw
        with patch.object(project.base, 'source_file', side_effect=altered):
            with self.assertRaisesRegex(ValueError, 'Focus scoped helper drift'):
                project.focus_proof(SOURCES)

    def test_manifest_hashes_exact_packet_members(self):
        packet = ROOT / project.OUTPUT
        manifest = json.loads((packet / 'package-manifest.json').read_text())
        actual = {str(path.relative_to(packet)): project.sha(path.read_bytes())
                  for path in packet.rglob('*') if path.is_file() and path.name != 'package-manifest.json'}
        self.assertEqual(manifest['files'], actual)


if __name__ == '__main__':
    unittest.main()
