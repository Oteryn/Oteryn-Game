import copy
import gzip
import json
from pathlib import Path
import tempfile
import unittest
import build_source_projection_progress as builder


class MonsterReviewTests(unittest.TestCase):
    def make_packet(self, folder):
        rows = []
        for i, status in enumerate(('SOURCE_SCHEMA_VALID', 'PARTIAL_TARGET_DEFINITIONS', 'BLOCKED')):
            rows.append({'slot_identity': {'candidate_id': 'fixture/monster', 'group': 'attacks', 'source_slot_index': i + 1},
                'source': 'canary', 'monster': 'fixture', 'original_slot_sha256': 'a' * 64, 'status': status,
                'full_slot_projection_complete': status == 'SOURCE_SCHEMA_VALID', 'blockers': [],
                'canonical_normalization': None, 'target_fragments': [], 'runtime_activation': False,
                'native_execution_qualified': False})
        path = folder / 'target-projections/projected-monster-slot-candidates.json.gz'
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(gzip.compress(json.dumps({'slots': rows}).encode(), mtime=0))
        return path, rows

    def test_partial_and_blocked_are_remaining_data_holds(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp); self.make_packet(root)
            manifest = {'counts': {'full_monster_slot_candidates': 1, 'partial_monster_slots': 1,
                                  'blocked_monster_slots': 1, 'target_definition_slots': 2}}
            view = builder.monster_view(root, root, manifest, {})
            self.assertEqual(2, view['remaining_data_holds'])
            self.assertEqual(3, view['runtime_unqualified_slot_count'])
            self.assertEqual(1, view['complete_target_slot_candidates'])

    def test_promotion_or_activation_is_rejected(self):
        for change in ({'full_slot_projection_complete': True}, {'runtime_activation': True}):
            with tempfile.TemporaryDirectory() as temp:
                root = Path(temp); path, rows = self.make_packet(root)
                rows[1].update(change)
                path.write_bytes(gzip.compress(json.dumps({'slots': rows}).encode()))
                manifest = {'counts': {'full_monster_slot_candidates': 1, 'partial_monster_slots': 1,
                                      'blocked_monster_slots': 1, 'target_definition_slots': 2}}
                with self.assertRaises(ValueError): builder.monster_view(root, root, manifest, {})

    def test_manifest_counts_cannot_override_actual_rows(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp); self.make_packet(root)
            manifest = {'counts': {'full_monster_slot_candidates': 2, 'partial_monster_slots': 0,
                                  'blocked_monster_slots': 1, 'target_definition_slots': 2}}
            with self.assertRaisesRegex(ValueError, 'counts differ'): builder.monster_view(root, root, manifest, {})

    def test_missing_imports_fail_before_planning_outputs(self):
        with tempfile.TemporaryDirectory() as temp:
            with self.assertRaises(FileNotFoundError): builder.build(Path(temp))
            self.assertEqual([], list(Path(temp).iterdir()))


class PrivateMonsterCompletionTests(unittest.TestCase):
    def make_case(self, root):
        baseline_folder = root / 'imports/spells/r54'
        path, original = MonsterReviewTests().make_packet(baseline_folder)
        for row in original:
            row['source_parameters'] = {'raw_flag': 'COMBAT_UNDEFINEDDAMAGE', 'minimum': -10, 'maximum': -20}
        path.write_bytes(gzip.compress(json.dumps({'slots': original}).encode(), mtime=0))
        manifest = {'counts': {'full_monster_slot_candidates': 1, 'partial_monster_slots': 1,
                              'blocked_monster_slots': 1, 'target_definition_slots': 2}}
        historical = builder.monster_view(root, baseline_folder, manifest, {})
        folder = root / 'imports/spells/r63'
        target = folder / 'monster-source-candidates'; target.mkdir(parents=True)
        rows = []
        for old in original[1:]:
            row = copy.deepcopy(old)
            row.update({'status': 'SOURCE_SCHEMA_VALID', 'full_slot_projection_complete': True,
                'target_schema_family': 'private_monster_slot_v2', 'authoring_contract_extension_pending': True,
                'source_consumer_implemented': False, 'input_provider_equivalence': False,
                'source_alias_to_existing_native_profile': False, 'required_operations_unrepresented': [],
                'controller': {'kind': 'typed_source_controller', 'parameters': {'original_error_flag': True}},
                **{name: False for name in builder.FLAGS}})
            rows.append(row)
        import_manifest = {'target_schema_family': 'private_monster_slot_v2', 'authoring_contract_extension_pending': True,
            'source_consumer_implemented': False, 'input_provider_equivalence': False,
            **{name: False for name in builder.FLAGS},
            'counts': {'records': 2, 'source_schema_valid': 2, 'full_source_data_slots': 2}}
        addition = (63, folder, import_manifest, {'manifest_path': 'imports/spells/r63/import-manifest.json'})
        self.write_rows(target, rows)
        return historical, addition, rows

    def write_rows(self, target, rows):
        (target / 'models.json').write_text(json.dumps({'schema': 'OTERYN_PRIVATE_MONSTER_SLOT_MODELS/v2', 'records': rows}))
        (target / 'slot-receipts.json').write_text(json.dumps({'records': [
            {**row, 'target_model_sha256': builder.sha(builder.canonical(row))} for row in rows]}))

    def test_exact_remaining_cohort_completes_data_and_preserves_original_full_slot(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); historical, addition, rows = self.make_case(root)
            view, worklist = builder.private_monster_overlay(root, historical, [addition])
            self.assertEqual(view['complete_target_slot_candidates'], 3)
            self.assertEqual(view['remaining_data_holds'], 0)
            self.assertEqual(view['runtime_unqualified_slot_count'], 3)
            self.assertEqual(worklist['records_count'], 2)
            old_full = historical['records'][0]
            self.assertEqual(next(r for r in view['records'] if r['slot_identity'] == old_full['slot_identity']), old_full)
            self.assertTrue(all(r['authoring_contract_extension_pending'] and not r['source_consumer_implemented'] for r in worklist['records']))
            self.assertEqual(rows[0]['source_parameters']['raw_flag'], 'COMBAT_UNDEFINEDDAMAGE')

    def test_duplicate_unknown_or_already_complete_identity_refused(self):
        for mode in ('duplicate', 'unknown', 'already_full'):
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory); historical, addition, rows = self.make_case(root)
                if mode == 'duplicate': rows[1]['slot_identity'] = copy.deepcopy(rows[0]['slot_identity'])
                elif mode == 'unknown': rows[0]['slot_identity']['candidate_id'] = 'unrelated'
                else: rows[0]['slot_identity'] = copy.deepcopy(historical['records'][0]['slot_identity'])
                self.write_rows(addition[1] / 'monster-source-candidates', rows)
                with self.assertRaises(ValueError): builder.private_monster_overlay(root, historical, [addition])

    def test_source_hash_or_literal_changes_refused_even_with_rehashed_receipt(self):
        for mode in ('hash', 'parameters'):
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory); historical, addition, rows = self.make_case(root)
                if mode == 'hash': rows[0]['original_slot_sha256'] = 'b' * 64
                else: rows[0]['source_parameters']['raw_flag'] = 'NORMALIZED_PHYSICAL'
                self.write_rows(addition[1] / 'monster-source-candidates', rows)
                with self.assertRaisesRegex(ValueError, 'changed original source facts'):
                    builder.private_monster_overlay(root, historical, [addition])

    def test_contract_provider_or_runtime_promotion_refused(self):
        for flag, value in [('source_consumer_implemented', True), ('input_provider_equivalence', True),
                            ('authoring_contract_extension_pending', False), ('runtime_activation', True)]:
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory); historical, addition, rows = self.make_case(root)
                rows[0][flag] = value; self.write_rows(addition[1] / 'monster-source-candidates', rows)
                with self.assertRaisesRegex(ValueError, 'cannot receive runtime qualification'):
                    builder.private_monster_overlay(root, historical, [addition])

    def test_missing_hold_or_forged_receipt_refused(self):
        for mode in ('missing', 'receipt'):
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory); historical, addition, rows = self.make_case(root)
                if mode == 'missing':
                    rows = rows[:1]; addition[2]['counts'] = {k: 1 for k in addition[2]['counts']}
                    self.write_rows(addition[1] / 'monster-source-candidates', rows)
                else:
                    path = addition[1] / 'monster-source-candidates/slot-receipts.json'
                    receipt = json.loads(path.read_text()); receipt['records'][0]['target_model_sha256'] = '0' * 64
                    path.write_text(json.dumps(receipt))
                with self.assertRaises(ValueError): builder.private_monster_overlay(root, historical, [addition])


if __name__ == '__main__': unittest.main()
