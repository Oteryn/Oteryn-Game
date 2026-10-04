import copy
import gzip
import json
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

import import_house_command_references as importer


class HouseCommandReferenceImportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data, cls.manifest = importer.prepare_import(importer.ROOT)
        cls.rows = list(map(json.loads, gzip.decompress(cls.data[cls.manifest['source_metadata_path']]).splitlines()))
        cls.callbacks = {r['registration_key']: r for r in map(json.loads, gzip.decompress((importer.ROOT / 'imports/spells/r28/player-source-bundles/source-callback-facts.jsonl.gz').read_bytes()).splitlines())}
        body = (importer.ROOT / 'docs/reference/tibia-manual/houses.md').read_bytes(); cls.cache = body.decode(); cls.cache_sha = importer.digest(body)

    def test_exact8_variants4_commands_and_no_verified_original_or_runtime_claim(self):
        counts = self.manifest['counts']
        self.assertEqual((counts['source_records'], counts['command_count']), (8, 4))
        self.assertEqual(counts['source_counts'], {'canary': 4, 'crystal': 4})
        self.assertEqual((counts['fresh_wiki_verified_records'], counts['original_manual_verified_records'], counts['full_spell_candidates']), (0, 0, 0))
        self.assertFalse(self.manifest['binding_qualified'])

    def test_forged_source_identity_or_command_is_refused(self):
        for field in ('sha256', 'git_blob'):
            rows = copy.deepcopy(self.rows); rows[0]['source'][field] = '0' * len(rows[0]['source'][field])
            with self.assertRaisesRegex(ValueError, 'HOUSE_REFERENCE_SOURCE_JOIN_MISMATCH'): importer.verify_rows(rows, self.callbacks, self.cache, self.cache_sha)
        rows = copy.deepcopy(self.rows); rows[0]['words'] = 'aleta sio'
        with self.assertRaisesRegex(ValueError, 'HOUSE_REFERENCE_SOURCE_JOIN_MISMATCH'): importer.verify_rows(rows, self.callbacks, self.cache, self.cache_sha)

    def test_changed_quote_or_verification_escalation_is_refused(self):
        rows = copy.deepcopy(self.rows); rows[0]['external_reference']['quote'] = 'invented official rule'
        with self.assertRaisesRegex(ValueError, 'HOUSE_REFERENCE_SECONDARY_QUOTE_MISMATCH'): importer.verify_rows(rows, self.callbacks, self.cache, self.cache_sha)
        rows = copy.deepcopy(self.rows); rows[0]['fresh_wiki_verified'] = True
        with self.assertRaisesRegex(ValueError, 'HOUSE_REFERENCE_VERIFICATION_ESCALATION_REFUSED'): importer.verify_rows(rows, self.callbacks, self.cache, self.cache_sha)

    def test_dropped_variant_or_rehashed_unqualified_receipt_is_refused(self):
        with self.assertRaisesRegex(ValueError, 'HOUSE_REFERENCE_SOURCE_POPULATION_MISMATCH'): importer.verify_rows(self.rows[:-1], self.callbacks, self.cache, self.cache_sha)
        with TemporaryDirectory() as directory:
            source = Path(directory); name = 'house-command-reference-receipt.json'; proof = json.loads(self.data['evidence/' + name]); proof['fresh_wiki_verified'] = True
            (source / name).write_text(json.dumps(proof))
            with self.assertRaisesRegex(ValueError, 'HOUSE_REFERENCE_QUALIFIED_PROOF_MISMATCH'): importer.prepare_import(importer.ROOT, source)

    def test_forged_output_and_original_manual_claim_refused_no_active_mutation(self):
        with TemporaryDirectory() as directory:
            root = Path(directory); target = root / 'imports/spells/r43'
            active = root / 'content/spells.manifest.json'; active.parent.mkdir(); active.write_bytes(b'active')
            old = root / 'imports/spells/r40/import-manifest.json'; old.parent.mkdir(parents=True); old.write_bytes(b'old')
            bad = copy.deepcopy(self.manifest); bad['original_manual_provenance_verified'] = True
            with self.assertRaisesRegex(ValueError, 'HOUSE_REFERENCE_VERIFICATION_CLAIM_REFUSED'): importer.write_import(root, target, self.data, bad)
            changed = dict(self.data); changed[self.manifest['source_metadata_path']] = b'forged'
            with self.assertRaisesRegex(ValueError, 'OUTPUT_PIN_MISMATCH'): importer.write_import(root, target, changed, self.manifest)
            importer.write_import(root, target, self.data, self.manifest)
            self.assertEqual(active.read_bytes(), b'active'); self.assertEqual(old.read_bytes(), b'old')
            with self.assertRaisesRegex(ValueError, 'IMPORT_SET_ALREADY_EXISTS'): importer.write_import(root, target, self.data, self.manifest)


if __name__ == '__main__': unittest.main()
