import copy
import hashlib
import json
import unittest
from unittest.mock import patch
import client_icons
from wheel_authoring import ROOT, read, validate_evidence


class ClientIconTests(unittest.TestCase):
    def setUp(self):
        self.candidate = read(ROOT / 'samples/wheel-candidate.json')
        self.manifest = read(ROOT / 'samples/client-icon-manifest.json')

    def test_every_reference_resolves_to_in_bounds_crop(self):
        client_icons.validate_manifest(self.manifest, self.candidate)
        self.assertEqual(len(self.manifest['bindings']), 520)
        self.assertEqual(len(self.manifest['icons']), 205)
        for pointer, reference in client_icons.references(self.candidate):
            entry = self.manifest['icons'][self.manifest['bindings'][pointer]]
            self.assertEqual(entry['source_index'], reference['source_index'])
            sheet = self.manifest['sheets'][entry['sheet']]
            x, y, width, height = entry['rect']
            self.assertEqual([y, width, height], [0, sheet['height'], sheet['height']])
            self.assertLessEqual(x + width, sheet['width'])

    def test_current_layout_does_not_use_older_client_slot(self):
        pointer = '/vocations/knight/slots/7/conviction/icon'
        self.assertEqual(self.manifest['bindings'][pointer], 'conviction:14')
        source = read(ROOT / 'samples/source-icon-reference.json')
        self.assertEqual(len(source['older_client_layout']['conviction_slot_differences']), 13)
        self.assertFalse(source['older_client_layout']['used_for_current_bindings'])

    def test_missing_binding_wrong_crop_wrong_sheet_and_admission_rejected(self):
        mutations = [lambda m: m['bindings'].pop(next(iter(m['bindings']))),
                     lambda m: m['icons']['conviction:14']['rect'].__setitem__(0, 390),
                     lambda m: m['sheets']['dedication'].update(sha256='0' * 64),
                     lambda m: m.update(runtime_admitted=True)]
        for mutate in mutations:
            with self.subTest(mutation=mutate):
                manifest = copy.deepcopy(self.manifest)
                mutate(manifest)
                with self.assertRaisesRegex(ValueError, 'ICON_MANIFEST_DRIFT'):
                    client_icons.validate_manifest(manifest)

    def test_sheet_too_small_rejected_before_manifest_generation(self):
        original_read = client_icons.read
        def small_sheet(path):
            source = original_read(path)
            if path.name == 'source-icon-reference.json':
                source['sheets'][0]['width'] = 16
            return source
        with patch.object(client_icons, 'read', side_effect=small_sheet):
            with self.assertRaisesRegex(ValueError, 'ICON_CROP_BOUNDS'):
                client_icons.build_manifest(self.candidate)

    def test_snapshot_and_reference_evidence_qualified(self):
        client_icons.validate_selection(self.candidate)
        validate_evidence(self.candidate, (ROOT / 'samples/wheel-candidate.json').read_bytes())

    def test_wrong_selected_value_rejected(self):
        correction = next(c for c in self.candidate['gems']['reference_corrections']
                          if c['key'] == 'augmented_mystic_repulse' and c['stage'] == 2)
        correction['selected_value'] = 15
        with self.assertRaisesRegex(ValueError, 'SELECTION_VALUE'):
            client_icons.validate_selection(self.candidate)

    def test_stale_source_sheet_digest_rejected(self):
        self.candidate['icon_evidence']['reference_sheet_sha256'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'ICON_SOURCE_DIGEST'):
            client_icons.build_manifest(self.candidate)

    def test_only_equal_original_cells_allow_reference_fallback(self):
        blocked = {key for key, entry in self.manifest['icons'].items()
                   if not entry['reference_fallback_verified']}
        self.assertEqual(blocked, {f'conviction:{i}' for i in (14,19,22,26,29,33,36,44,45)} |
                         {'revelation:8'} | {f'supreme_mod:{i}' for i in (36,37,77,88,89,90)})

    def test_custom_candidate_and_serialized_bytes_are_bound(self):
        self.candidate['revision'] = 'another-candidate'
        serialized = json.dumps(self.candidate).encode()
        manifest = client_icons.build_manifest(self.candidate, serialized)
        self.assertEqual(manifest['candidate_sha256'], hashlib.sha256(serialized).hexdigest())
        with self.assertRaisesRegex(ValueError, 'ICON_MANIFEST_DRIFT'):
            client_icons.validate_manifest(self.manifest, self.candidate, serialized)
        with self.assertRaisesRegex(ValueError, 'ICON_CANDIDATE_CONTENT'):
            client_icons.build_manifest(self.candidate, b'{}')

    def test_original_sheet_bounds_and_complete_comparison_required(self):
        original_read = client_icons.read
        for mutation, code in [('width', 'ICON_ORIGINAL_CROP'), ('coverage', 'ICON_COMPARISON_COVERAGE')]:
            def broken(path):
                source = original_read(path)
                if path.name == 'source-icon-reference.json':
                    original = source['sheets'][0]['original_cdn']
                    if mutation == 'width':
                        original.update(width=16, equal_reference_cells=[0])
                    else:
                        original['equal_reference_cells'].pop()
                return source
            with self.subTest(mutation=mutation), patch.object(client_icons, 'read', side_effect=broken):
                with self.assertRaisesRegex(ValueError, code):
                    client_icons.build_manifest(self.candidate)
