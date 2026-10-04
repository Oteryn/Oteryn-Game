import copy
import gzip
import json
from pathlib import Path
import shutil
from tempfile import TemporaryDirectory
import unittest

import import_bounded_source_evidence as importer


class BoundedEvidenceImportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.prepared = {r: importer.prepare_import(importer.ROOT, r) for r in (32, 33, 35, 36)}

    def test_actual_family_counts_and_no_new_spell_candidates(self):
        for revision, expected in ((32, 1), (33, 59), (35, 3), (36, 100)):
            manifest = self.prepared[revision][1]
            self.assertEqual(manifest['counts']['source_records'], expected)
            self.assertEqual(manifest['full_spell_candidates'], 0)
            self.assertFalse(manifest['runtime_activation'])
        self.assertEqual(self.prepared[33][1]['counts']['ordered_parameter_assignments'], 146)
        self.assertEqual(self.prepared[36][1]['counts']['fourth_argument_counts'], {'omitted_nil': 74, 'source_symbol': 26})

    def test_no_raw_lua_is_distributed_and_swift_reference_has_own_schema(self):
        data, manifest = self.prepared[32]
        self.assertFalse(any(name.endswith('.lua') for name in data))
        entry = next(e for e in manifest['artifacts'] if e['path'] == 'evidence/source-reference.json')
        self.assertEqual(entry['schemaRefs'], ['evidence/source-reference.schema.json'])

    def rewrite_gzip(self, revision, folder, change):
        _, artifact, proof_name, _ = importer.FAMILIES[revision]
        folder.mkdir()
        source = importer.ROOT / ('docs/reference/spells/r' + str(revision) + '-source-closure')
        shutil.copyfile(source / proof_name, folder / proof_name)
        if revision == 36:
            shutil.copyfile(source / 'source-conjure-helper.schema.json', folder / 'source-conjure-helper.schema.json')
        value = gzip.decompress((source / artifact).read_bytes())
        records = [json.loads(line) for line in value.splitlines()] if revision == 33 else json.loads(value)
        change(records)
        payload = b''.join(json.dumps(r, sort_keys=True).encode() + b'\n' for r in records) if revision == 33 else json.dumps(records).encode()
        body = gzip.compress(payload, mtime=0); (folder / artifact).write_bytes(body)
        proof = json.loads((folder / proof_name).read_bytes())
        proof['payload_sha256'] = importer.digest(payload)
        proof['data_sha256' if revision == 36 else 'gzip_sha256'] = importer.digest(body)
        (folder / proof_name).write_bytes(importer.encoded(proof))

    def test_rehashed_condition_parameter_value_forgery_is_refused(self):
        with TemporaryDirectory() as directory:
            folder = Path(directory) / 'source'
            def change(rows):
                row = next(r for r in rows if any(a['value']['kind'] == 'scalar' for a in r['parameter_assignments']))
                next(a for a in row['parameter_assignments'] if a['value']['kind'] == 'scalar')['value']['value'] = 999999
            self.rewrite_gzip(33, folder, change)
            with self.assertRaisesRegex(ValueError, 'CONDITION_PARAMETER_CALL_LINK_MISMATCH'):
                importer.prepare_import(importer.ROOT, 33, folder)

    def test_rehashed_conjure_output_parameter_forgery_is_refused(self):
        # Keep the schema in the repository path so its archived relative sourcePath remains valid.
        with TemporaryDirectory(dir=importer.ROOT / 'tools/content-migration') as directory:
            folder = Path(directory) / 'source'
            self.rewrite_gzip(36, folder, lambda rows: rows[0].__setitem__('count', rows[0]['count'] + 1))
            with self.assertRaisesRegex(ValueError, 'CONJURE_HELPER_PARAMETERS_MISMATCH'):
                importer.prepare_import(importer.ROOT, 36, folder)

    def test_tampered_import_bytes_are_refused_before_output(self):
        data, manifest = self.prepared[33]
        changed = dict(data); changed[manifest['source_metadata_path']] = b'tampered'
        with TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaisesRegex(ValueError, 'OUTPUT_PIN_MISMATCH'):
                importer.write_import(root, root / 'imports/spells/r33', changed, manifest)
            self.assertFalse((root / 'imports/spells/r33').exists())

    def test_full_candidate_and_activation_claims_are_refused(self):
        data, manifest = self.prepared[35]
        with TemporaryDirectory() as directory:
            root = Path(directory)
            for field, error in [('full_spell_candidates', 'EVIDENCE_CANDIDATE_CLAIM_REFUSED'), ('runtime_activation', 'ACTIVE_OR_EQUIVALENT_CLAIM_REFUSED')]:
                bad = copy.deepcopy(manifest); bad[field] = True
                with self.assertRaisesRegex(ValueError, error):
                    importer.write_import(root, root / 'imports/spells/r35', data, bad)

    def test_new_family_sets_preserve_active_content_and_previous_sets_and_refuse_replay(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            old = root / 'imports/spells/r28/import-manifest.json'; old.parent.mkdir(parents=True); old.write_bytes(b'old import')
            active = root / 'content/spells.manifest.json'; active.parent.mkdir(parents=True); active.write_bytes(b'active input')
            for revision, (data, manifest) in self.prepared.items():
                target = root / ('imports/spells/r' + str(revision))
                importer.write_import(root, target, data, manifest)
                with self.assertRaisesRegex(ValueError, 'IMPORT_SET_ALREADY_EXISTS'):
                    importer.write_import(root, target, data, manifest)
            self.assertEqual(old.read_bytes(), b'old import'); self.assertEqual(active.read_bytes(), b'active input')


if __name__ == '__main__': unittest.main()
