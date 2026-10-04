import copy
import gzip
import json
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

import import_cached_monster_references as importer


class CachedMonsterReferenceImportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls): cls.data, cls.manifest = importer.prepare_import(importer.ROOT)

    def test_actual_cohorts_and_no_override_or_candidate_inflation(self):
        counts = self.manifest['counts']
        self.assertEqual([counts[k] for k in ('fact_records', 'profile_records', 'slot_records', 'missing_target_records')], [1869, 5237, 20742, 1757])
        self.assertEqual(counts['normalized_attack_observations'], 5680)
        self.assertEqual(counts['all_field_verified_slots'], 0)
        self.assertEqual(counts['full_spell_candidates'], 0)
        self.assertFalse(self.manifest['source_override'])
        self.assertEqual(counts['cached_page_statuses'], {'attack_facts_read': 1736, 'unavailable': 133})

    def test_rehashed_proof_or_unreviewed_inputs_refused(self):
        with TemporaryDirectory() as directory:
            source = Path(directory); name = 'cached-monster-spell-reference-proof.json'
            proof = json.loads(self.data['evidence/' + name]); proof['cache_inputs'][0]['sha256'] = '0' * 64
            (source / name).write_text(json.dumps(proof))
            with self.assertRaisesRegex(ValueError, 'CACHED_REFERENCE_QUALIFIED_PROOF_MISMATCH'): importer.prepare_import(importer.ROOT, source)

    def fixture(self):
        provenance = {'revision': 'r', 'path': 'monster.lua', 'sha256': 'h', 'git_blob': 'b'}
        profile = {'candidate_id': 'candidate', 'name': 'Rat', 'source': 'canary', 'provenance': provenance}
        slot = {'candidate_id': 'candidate', 'source': 'canary', 'monster': 'Rat', 'monster_source': provenance, 'group': 'attacks', 'source_slot_index': 1,
                'source_parameters': {'chance': 100}, 'conversion_status': 'mapped'}
        key = importer.slot_key(slot); fact = {'monster_name': 'Rat', 'url': 'https://example/Rat', 'page_sha256': 'p', 'status': 'attack_facts_read', 'attacks': []}
        comparison = {'slot_key': key, 'source_sha256': 'h', 'file': 'monster.lua', 'source': 'canary', 'block': 'attacks', 'field_comparisons': {'chance': {'source_value': 100, 'status': 'unknown_external_not_published'}},
                      'external_url': fact['url'], 'external_page_sha256': 'p', 'external_status': fact['status'], 'all_fields_verified': False}
        row = {'slot_identity': {'candidate_id': 'candidate', 'group': 'attacks', 'source_slot_index': 1}, 'source_identity': provenance, 'source': 'canary', 'monster_name': 'Rat',
               'source_slot_sha256': importer.digest(importer.canonical(slot)), 'source_conversion_status': 'mapped', 'cached_comparison': comparison,
               'all_fields_verified': False, 'unknown_fields': ['chance'], 'source_uncertainty_fields': ['chance']}
        groups = {'fact': [{'fact': fact}], 'slot': [row], 'profile': [{'candidate_id': 'candidate', 'source': 'canary', 'monster_name': 'Rat', 'source_identity': provenance,
                  'cached_page_url': fact['url'], 'cached_page_sha256': 'p', 'cached_page_status': fact['status'], 'slot_keys': [key]}],
                  'missing_target': [{'monster_name': 'Rat', 'candidate_ids': ['candidate'], 'source_uncertainty_fields': ['chance'], 'captured_url': fact['url'], 'page_status': fact['status'],
                                      'reason': 'engine_parameters_and_unique_slot_identity_not_verified'}]}
        return groups, [profile], [slot], [fact], [comparison]

    def test_source_slot_forgery_and_verification_escalation_refused(self):
        args = self.fixture(); importer.validate_rows(*args)
        args[0]['slot'][0]['source_slot_sha256'] = 'changed'
        with self.assertRaisesRegex(ValueError, 'CACHED_REFERENCE_SOURCE_SLOT_CHANGED'): importer.validate_rows(*args)
        args = self.fixture(); args[0]['slot'][0]['all_fields_verified'] = True
        with self.assertRaisesRegex(ValueError, 'CACHED_REFERENCE_SOURCE_SLOT_CHANGED'): importer.validate_rows(*args)

    def test_cached_observation_cannot_override_engine_parameter(self):
        args = self.fixture(); args[-1][0]['field_comparisons']['chance']['source_value'] = 99
        with self.assertRaisesRegex(ValueError, 'CACHED_REFERENCE_COMPARISON_PROVENANCE_MISMATCH'): importer.validate_rows(*args)

    def test_unknown_fields_and_missing_target_cohort_preserved(self):
        args = self.fixture(); args[0]['slot'][0]['unknown_fields'] = []
        with self.assertRaisesRegex(ValueError, 'CACHED_REFERENCE_UNCERTAINTY_CHANGED'): importer.validate_rows(*args)
        args = self.fixture(); args[0]['missing_target'] = []
        with self.assertRaisesRegex(ValueError, 'CACHED_REFERENCE_MISSING_TARGET_COHORT_MISMATCH'): importer.validate_rows(*args)

    def test_output_hash_and_override_claim_refused_and_prior_sets_preserved(self):
        with TemporaryDirectory() as directory:
            root = Path(directory); target = root / 'imports/spells/r41'
            active = root / 'content/spells.manifest.json'; active.parent.mkdir(); active.write_bytes(b'active')
            old = root / 'imports/spells/r39/import-manifest.json'; old.parent.mkdir(parents=True); old.write_bytes(b'old')
            bad = copy.deepcopy(self.manifest); bad['source_override'] = True
            with self.assertRaisesRegex(ValueError, 'CACHED_REFERENCE_OVERRIDE_CLAIM_REFUSED'): importer.write_import(root, target, self.data, bad)
            changed = dict(self.data); changed[self.manifest['source_metadata_path']] = b'forged'
            with self.assertRaisesRegex(ValueError, 'OUTPUT_PIN_MISMATCH'): importer.write_import(root, target, changed, self.manifest)
            importer.write_import(root, target, self.data, self.manifest)
            self.assertEqual(active.read_bytes(), b'active'); self.assertEqual(old.read_bytes(), b'old')
            with self.assertRaisesRegex(ValueError, 'IMPORT_SET_ALREADY_EXISTS'): importer.write_import(root, target, self.data, self.manifest)


if __name__ == '__main__': unittest.main()
