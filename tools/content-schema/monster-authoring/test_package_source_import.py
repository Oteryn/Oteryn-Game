import unittest
from tempfile import TemporaryDirectory
from pathlib import Path

import package_source_import as package


class PackageSourceImportTests(unittest.TestCase):
    def test_code_repository_does_not_depend_on_output_location(self):
        self.assertEqual(package.code_repository(), Path(__file__).resolve().parents[3])
        self.assertTrue((package.code_repository() / 'AGENTS.md').is_file())

    def test_unselected_registered_variant_still_requires_conversion(self):
        with self.assertRaisesRegex(ValueError, 'lacks conversion facts'):
            package.registration_capture_outcomes([{'name': 'duplicate', 'resolution_selected': False, 'conversion': None}])

    def test_captured_variants_and_explicit_failures_remain_distinct(self):
        rows = [
            {'conversion': {'spell_calls': {}, 'combats': {}}},
            {'conversion': {'native_conversion': 'unresolved_callback'}},
            {'conversion': {'error': 'unsupported actual source operation'}},
            {'conversion': None, 'capture_error': 'source constructor unavailable'},
        ]
        self.assertEqual(package.registration_capture_outcomes(rows), {
            'captured_conversion_facts': 2, 'explicit_capture_error': 1, 'explicit_conversion_error': 1,
        })

    def test_empty_or_metadata_only_conversion_is_not_capture(self):
        for conversion in ({}, {'name': 'uncaptured'}, 'opaque'):
            with self.subTest(conversion=conversion), self.assertRaises(ValueError):
                package.registration_capture_outcomes([{'conversion': conversion}])

    def test_blank_error_does_not_qualify_uncaptured_variant(self):
        with self.assertRaises(ValueError):
            package.registration_capture_outcomes([{'conversion': None, 'error': ' '}])

    def test_converter_fingerprints_reject_code_drift(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            code = root / 'converter.py'
            code.write_bytes(b'original')
            pins = {'converter.py': package.digest(code.read_bytes())}
            summary = {'converter_inputs_sha256': pins}
            self.assertEqual(package.verify_converter_inputs(summary, root), pins)
            code.write_bytes(b'changed')
            with self.assertRaisesRegex(ValueError, 'fingerprint mismatch'):
                package.verify_converter_inputs(summary, root)

    def test_converter_fingerprints_require_owned_relative_paths(self):
        with TemporaryDirectory() as directory:
            for name in ('../outside.py', '/outside.py'):
                with self.subTest(name=name), self.assertRaisesRegex(ValueError, 'escapes code repository'):
                    package.verify_converter_inputs({'converter_inputs_sha256': {name: '0' * 64}}, Path(directory))

    def test_missing_converter_fingerprints_fail_closed(self):
        with self.assertRaisesRegex(ValueError, 'lacks converter input fingerprints'):
            package.verify_converter_inputs({}, package.code_repository())


if __name__ == '__main__':
    unittest.main()
