"""Reject changes beyond the five sealed comment-only storage writes."""
import copy
import json
import unittest
from pathlib import Path

import source_comment_write_guard as guard
from source_fix_guard import normalize_core


class CommentWriteGuardTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.snapshots = json.loads((Path(__file__).parent / 'samples/comment-write-fix/snapshots.json').read_text())

    def core(self, row, side):
        return {'identity': {'key': row['quest'], 'revision': 'quest-r1'},
                'source_data': {'progress': copy.deepcopy(row[side + '_tracks'])},
                'claims': [], 'requirements': {'min_level': 0},
                'missing_data': [], 'reported_source_readiness': None}

    def test_exact_three_owner_capsules(self):
        self.assertEqual(len(self.snapshots), 3)
        for row in self.snapshots:
            with self.subTest(quest=row['quest']):
                old, new = guard.reviewed_pair(self.core(row, 'old'), self.core(row, 'new'))
                self.assertEqual(old, new)

    def test_unchanged_earlier_pair_remains_visible(self):
        for row in self.snapshots:
            old = self.core(row, 'old')
            new = copy.deepcopy(old)
            new['requirements']['min_level'] = 999
            a, b = guard.reviewed_pair(old, new)
            self.assertEqual(a['source_data']['progress'], old['source_data']['progress'])
            self.assertNotEqual(normalize_core(a), normalize_core(b))

    def test_wrong_owner_and_identity(self):
        row = self.snapshots[0]
        old, new = self.core(row, 'old'), self.core(row, 'new')
        new['identity']['key'] = 'oteryn:quest.unrelated'
        with self.assertRaises(ValueError):
            guard.reviewed_pair(old, new)

    def test_any_old_occurrence_pin_change_is_rejected(self):
        for row in self.snapshots:
            for field, value in [('target', '99999'), ('line', 999), ('path', 'fake.lua'),
                                 ('revision', '0' * 40), ('blob_sha256', '0' * 64),
                                 ('line_sha256', '0' * 64)]:
                with self.subTest(quest=row['quest'], field=field):
                    old, new = self.core(row, 'old'), self.core(row, 'new')
                    old['source_data']['progress'][0]['transitions'][0]['source_occurrences'][0][field] = value
                    with self.assertRaises(ValueError):
                        guard.reviewed_pair(old, new)

    def test_real_chagorz_write_and_count_are_sealed(self):
        row = next(r for r in self.snapshots if r['quest'].endswith('rotten_blood_quest'))
        for field in ('transitions', 'writes'):
            with self.subTest(field=field):
                old, new = self.core(row, 'old'), self.core(row, 'new')
                if field == 'transitions':
                    new['source_data']['progress'][0]['transitions'].clear()
                else:
                    new['source_data']['progress'][0]['writes']['crystalserver'] = 2
                with self.assertRaises(ValueError):
                    guard.reviewed_pair(old, new)

    def test_unrelated_progress_claims_requirements_and_native_fields_stay_visible(self):
        for row in self.snapshots:
            for field in ('progress', 'claims', 'requirements', 'native_lowering'):
                with self.subTest(quest=row['quest'], field=field):
                    old, new = self.core(row, 'old'), self.core(row, 'new')
                    if field == 'progress':
                        new['source_data']['progress'].append({'key': 'real/thirteen/presente', 'transitions': []})
                    elif field == 'claims':
                        new['claims'].append({'key': 'fake'})
                    elif field == 'requirements':
                        new['requirements']['min_level'] = 999
                    else:
                        new['native_lowering'] = {'state': 'READY'}
                    a, b = guard.reviewed_pair(old, new)
                    self.assertNotEqual(normalize_core(a), normalize_core(b))


if __name__ == '__main__':
    unittest.main()
