import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import import_server_data as importer
from wheel_authoring import ROOT, read


class ServerImportTests(unittest.TestCase):
    def test_committed_import_matches_the_qualified_source(self):
        importer.check_or_write(True)

    def test_projection_preserves_all_prepared_data_and_owner_cost(self):
        source = read(ROOT / 'samples/wheel-candidate.json')
        result = {k: json.loads(v) for k, v in importer.exports().items()}
        wheel, gems = result['wheel.json'], result['gems.json']
        self.assertFalse(wheel['runtime_admitted'])
        self.assertFalse(gems['runtime_admitted'])
        self.assertEqual(wheel['data']['progression'], source['progression'])
        self.assertEqual(wheel['data']['topology'], source['topology'])
        for vocation, fields in source['vocations'].items():
            self.assertEqual({**wheel['data']['vocations'][vocation],
                              **gems['data']['vocations'][vocation]}, fields)
        self.assertEqual(gems['data']['catalogue'], source['gems'])
        self.assertEqual(gems['data']['catalogue']['grade_costs'][1]['supreme'],
                         {'gold': 12000000, 'fragments': 15})
        self.assertEqual(gems['data']['catalogue']['atelier']['operation_policy']['revealed_gem_limit'], 225)

    def test_candidate_with_unqualified_price_cannot_be_exported(self):
        original = importer.read
        def changed(path):
            value = original(path)
            if path.name == 'wheel-candidate.json':
                value = copy.deepcopy(value)
                value['gems']['grade_costs'][1]['supreme']['gold'] = 12500000
            return value
        with patch.object(importer, 'read', side_effect=changed):
            with self.assertRaisesRegex(ValueError, 'EVIDENCE_CANDIDATE_CONTENT'):
                importer.exports()

    def test_check_refuses_missing_or_drifting_data_without_writing(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(importer, 'DIRECTORY', Path(directory)):
            with self.assertRaisesRegex(ValueError, 'SERVER_IMPORT_DRIFT'):
                importer.check_or_write(True)
            importer.check_or_write()
            importer.check_or_write(True)
            path = Path(directory) / 'gems.json'
            value = path.read_text().replace('12000000', '12500000', 1)
            path.write_text(value)
            with self.assertRaisesRegex(ValueError, 'SERVER_IMPORT_DRIFT: gems.json'):
                importer.check_or_write(True)
            self.assertEqual(path.read_text(), value)

    def test_line_ending_drift_cannot_pass_a_digest_bound_import(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(importer, 'DIRECTORY', Path(directory)):
            importer.check_or_write()
            path = Path(directory) / 'wheel.json'
            path.write_bytes(path.read_bytes().replace(b'\n', b'\r\n'))
            with self.assertRaisesRegex(ValueError, 'SERVER_IMPORT_DRIFT: wheel.json'):
                importer.check_or_write(True)
