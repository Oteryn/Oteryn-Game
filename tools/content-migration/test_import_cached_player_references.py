import copy
import gzip
import json
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

import import_cached_player_references as importer


class CachedPlayerReferenceImportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls): cls.data, cls.manifest = importer.prepare_import(importer.ROOT)

    def test_actual483_source773_pages485_supplements_and_frozen308_baseline(self):
        counts = self.manifest['counts']
        self.assertEqual([counts[k] for k in ('source_records', 'wiki_page_fact_records', 'supplemental_source_absent_fields', 'candidate_records_unchanged')], [483, 773, 485, 308])
        self.assertEqual([counts[k] for k in ('source_records_with_context_compatible_page', 'source_records_without_context_compatible_page')], [469, 14])
        self.assertEqual(counts['field_comparisons'], 5796)
        self.assertEqual(counts['full_spell_candidates'], 0)
        self.assertFalse(self.manifest['source_override'])
        self.assertNotIn('imports/spells/r42/import-manifest.json', self.manifest['reference_inputs'])

    def fixture(self):
        groups = {group: [json.loads(line) for line in gzip.decompress(self.data['evidence/' + name]).splitlines()] for name, (group, _) in importer.OUTPUTS.items()}
        row = next(r for r in groups['comparisons'] if r['matching_evidence']); reg = row['registration_key']
        groups['comparisons'] = [row]; groups['supplemental_facts'] = [r for r in groups['supplemental_facts'] if r['registration_key'] == reg]
        base = importer.ROOT / 'imports/spells/r28/player-source-bundles'
        callbacks = {r['registration_key']: r for r in map(json.loads, gzip.decompress((base / 'source-callback-facts.jsonl.gz').read_bytes()).splitlines())}
        body = (base / reg.split('/')[0] / importer.digest(reg.encode())[:16] / 'source-header.json').read_bytes()
        return groups, {reg: (body, json.loads(body)['spell'])}, {reg: callbacks[reg]}, {reg: row['candidate_status']}, {reg: row['candidate_overlays']}

    def test_header_sha_or_source_identity_forgery_refused(self):
        for field in ('source_sha256', 'source_header_sha256'):
            args = self.fixture(); args[0]['comparisons'][0][field] = '0' * 64
            with self.assertRaisesRegex(ValueError, 'PLAYER_REFERENCE_FROZEN_SOURCE_JOIN_MISMATCH'): importer.verify_source_joins(*args)

    def test_rune_context_and_identity_anchor_forgery_refused(self):
        args = self.fixture(); args[0]['comparisons'][0]['context'] = 'spell_cast'
        with self.assertRaisesRegex(ValueError, 'PLAYER_REFERENCE_SOURCE_CONTEXT_MISMATCH'): importer.verify_source_joins(*args)
        args = self.fixture(); args[0]['comparisons'][0]['matching_evidence'][0]['identity_anchor_page_keys'] = ['0' * 64]
        with self.assertRaisesRegex(ValueError, 'PLAYER_REFERENCE_IDENTITY_ANCHOR_MISMATCH'): importer.verify_source_joins(*args)

    def test_supplemental_evidence_or_source_override_escalation_refused(self):
        args = self.fixture(); importer.verify_source_joins(*args)
        args[0]['supplemental_facts'][0]['source_values_overwritten'] = True
        with self.assertRaisesRegex(ValueError, 'PLAYER_REFERENCE_SUPPLEMENTAL_OVERRIDE_MISMATCH'): importer.verify_source_joins(*args)
        args = self.fixture(); args[0]['supplemental_facts'].pop()
        with self.assertRaisesRegex(ValueError, 'PLAYER_REFERENCE_SUPPLEMENTAL_COHORT_MISMATCH'): importer.verify_source_joins(*args)

    def test_unreviewed_rehashed_source_package_is_refused(self):
        with TemporaryDirectory() as directory:
            source = Path(directory); manifest = json.loads(self.data['evidence/package-manifest.json']); manifest['files']['coverage.json'] = '0' * 64
            (source / 'package-manifest.json').write_text(json.dumps(manifest))
            with self.assertRaisesRegex(ValueError, 'PLAYER_REFERENCE_QUALIFIED_PACKAGE_MISMATCH'): importer.prepare_import(importer.ROOT, source)

    def test_new_import_does_not_mutate_old_sets_or_active_content(self):
        with TemporaryDirectory() as directory:
            root = Path(directory); target = root / 'imports/spells/r40'
            active = root / 'content/spells.manifest.json'; active.parent.mkdir(); active.write_bytes(b'active')
            old = root / 'imports/spells/r41/import-manifest.json'; old.parent.mkdir(parents=True); old.write_bytes(b'old')
            bad = copy.deepcopy(self.manifest); bad['source_override'] = True
            with self.assertRaisesRegex(ValueError, 'PLAYER_REFERENCE_OVERRIDE_CLAIM_REFUSED'): importer.write_import(root, target, self.data, bad)
            changed = dict(self.data); changed[self.manifest['source_metadata_path']] = b'forged'
            with self.assertRaisesRegex(ValueError, 'OUTPUT_PIN_MISMATCH'): importer.write_import(root, target, changed, self.manifest)
            importer.write_import(root, target, self.data, self.manifest)
            self.assertEqual(active.read_bytes(), b'active'); self.assertEqual(old.read_bytes(), b'old')
            with self.assertRaisesRegex(ValueError, 'IMPORT_SET_ALREADY_EXISTS'): importer.write_import(root, target, self.data, self.manifest)


if __name__ == '__main__': unittest.main()
