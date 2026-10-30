"""The Monk amendment cannot hide mutations even with a resealed core signature."""
import copy
import unittest
from pathlib import Path

import source_fix_guard as guard
from source_monk_native_guard import CORE, EVIDENCE


class MonkSourceGuardTests(unittest.TestCase):
    def test_exact_pair_and_resealed_mutants(self):
        root = Path(__file__).resolve().parents[3]
        receipt, approval = guard.verified_receipt(root)
        baseline = guard.read_json(root / (guard.DIRECTORY + 'baseline-core-digests.json'))
        original = next(c for c in receipt['changes'] if c['key'] == CORE)
        guard.validate_change(original, baseline, original['new_core'], approval, root)
        mutations = (
            lambda c: c['native_lowering'].update(state='WAITING_IMPLEMENTATION'),
            lambda c: c['native_lowering']['canonical_interaction_refs'].pop(),
            lambda c: c['native_lowering']['canonical_progress_refs'].append('invented'),
            lambda c: c['native_lowering'].update(reason='unreviewed'),
            lambda c: c.update(requirements={'min_level': 999999}),
            lambda c: c['source_refs']['quest'].update(revision='unreviewed'),
        )
        for mutation in mutations:
            with self.subTest(mutation=mutation):
                change, resealed = copy.deepcopy(original), copy.deepcopy(approval)
                mutation(change['new_core'])
                change['to_digest'] = guard.digest(change['new_core'])
                for row in resealed['approved_core_digests']:
                    if row['key'] == CORE:
                        row['to_digest'] = change['to_digest']
                with self.assertRaises(ValueError):
                    guard.validate_change(change, baseline, change['new_core'], resealed, root)

    def test_evidence_is_hash_pinned(self):
        root = Path(__file__).resolve().parents[3]
        receipt, _ = guard.verified_receipt(root)
        descriptor = next(r for r in receipt['proof_inputs'] if r['path'] == EVIDENCE)
        guard.verify_file(root, descriptor)
        descriptor = dict(descriptor, sha256='0' * 64)
        with self.assertRaises(ValueError):
            guard.verify_file(root, descriptor)
