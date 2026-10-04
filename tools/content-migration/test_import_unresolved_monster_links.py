import copy
import gzip
import json
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

import import_unresolved_monster_links as importer


class UnresolvedMonsterLinkImportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data, cls.manifest = importer.prepare_import(importer.ROOT)

    def test_exact_175_unresolved_source_slots_and_false_admission(self):
        counts = self.manifest['counts']
        self.assertEqual(counts['source_records'], 175)
        self.assertEqual(counts['source_counts'], {'canary': 54, 'crystal': 121})
        self.assertEqual(counts['registered_fact_links'], 156)
        self.assertEqual(counts['inline_raw_slots'], 19)
        self.assertEqual(counts['custom_descriptor_links'], 21)
        self.assertEqual(counts['status_counts'], {'unresolved_semantics': 175})
        self.assertFalse(self.manifest['native_ability_admission'])

    def forged_packet(self, folder, change):
        source = importer.ROOT / 'docs/reference/spells/r37-source-closure'
        name, proof_name = 'unresolved-monster-spell-links.jsonl.gz', 'unresolved-monster-spell-links-proof.json'
        rows = [json.loads(line) for line in gzip.decompress((source / name).read_bytes()).splitlines()]
        change(rows)
        payload = b''.join(importer.canonical(r) + b'\n' for r in rows); compressed = gzip.compress(payload, mtime=0)
        proof = json.loads((source / proof_name).read_bytes()); proof['payload_sha256'] = importer.digest(payload); proof['gzip_sha256'] = importer.digest(compressed)
        (folder / name).write_bytes(compressed); (folder / proof_name).write_bytes(importer.encoded(proof))

    def test_rehashed_slot_parameter_forgery_is_refused(self):
        with TemporaryDirectory() as directory:
            folder = Path(directory)
            self.forged_packet(folder, lambda rows: rows[0]['source_parameters'].__setitem__('chance', 99))
            with self.assertRaisesRegex(ValueError, 'MONSTER_LINK_ORIGINAL_SLOT_CHANGED'):
                importer.prepare_import(importer.ROOT, folder)

    def test_rehashed_inventory_link_identity_forgery_is_refused(self):
        with TemporaryDirectory() as directory:
            folder = Path(directory)
            self.forged_packet(folder, lambda rows: rows[0]['registered_fact_link'].__setitem__('record_index', 0))
            with self.assertRaisesRegex(ValueError, 'MONSTER_LINK_FACT_JOIN_MISMATCH'):
                importer.prepare_import(importer.ROOT, folder)

    def test_tampered_output_and_admission_claims_refused(self):
        with TemporaryDirectory() as directory:
            root = Path(directory); destination = root / 'imports/spells/r37'
            changed = dict(self.data); changed[self.manifest['source_metadata_path']] = b'changed'
            with self.assertRaisesRegex(ValueError, 'OUTPUT_PIN_MISMATCH'):
                importer.write_import(root, destination, changed, self.manifest)
            bad = copy.deepcopy(self.manifest); bad['native_ability_admission'] = True
            with self.assertRaisesRegex(ValueError, 'MONSTER_LINK_ACTIVATION_REFUSED'):
                importer.write_import(root, destination, self.data, bad)
            self.assertFalse(destination.exists())

    def test_new_import_preserves_old_sets_and_active_content(self):
        with TemporaryDirectory() as directory:
            root = Path(directory); destination = root / 'imports/spells/r37'
            active = root / 'content/spells.manifest.json'; active.parent.mkdir(); active.write_bytes(b'active')
            old = root / 'imports/spells/r28/import-manifest.json'; old.parent.mkdir(parents=True); old.write_bytes(b'old')
            importer.write_import(root, destination, self.data, self.manifest)
            self.assertEqual(active.read_bytes(), b'active'); self.assertEqual(old.read_bytes(), b'old')
            with self.assertRaisesRegex(ValueError, 'IMPORT_SET_ALREADY_EXISTS'):
                importer.write_import(root, destination, self.data, self.manifest)


if __name__ == '__main__': unittest.main()
