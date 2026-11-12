import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import native_kill_goals as native


class NativeSourceGoalProof(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name)
        self.cohort = json.loads((native.COHORTS / 'elvenbane.json').read_bytes())

    def document(self, cohort):
        (self.directory / 'elvenbane.json').write_text(json.dumps(cohort))
        with patch.object(native, 'COHORTS', self.directory):
            return native.document()

    def test_six_exact_source_origins_never_substitute_for_each_other(self):
        goals = [g for g in self.document(self.cohort)['goals'] if '/elvenbane-' in g['key']]
        self.assertEqual(len(goals), 6)
        self.assertEqual({g['count'] for g in goals}, {1})
        self.assertEqual(len({tuple(g['origins'][0][field] for field in
                                   ('creature', 'x', 'y', 'floor')) for g in goals}), 6)
        for goal in goals:
            expected = [] if goal['counter'].endswith('/dragon') else [
                {'counter': 'oteryn:quest-progress/elvenbane/native-defenders/dragon', 'count': 1}]
            self.assertEqual(goal['requires'], expected)

    def test_source_drift_foreign_path_and_neighbor_species_or_cell_refuse(self):
        for mutation in ('hash', 'path', 'pointer', 'species', 'cell'):
            changed = copy.deepcopy(self.cohort)
            goal = changed['goals'][0]
            if mutation == 'hash':
                goal['sources'][0]['sha256'] = '0' * 64
            elif mutation == 'path':
                goal['sources'][0]['path'] = '../outside'
            elif mutation == 'pointer':
                goal['origins'][0]['source_pointer'] = '/records/999999/declaration/points/0'
            elif mutation == 'species':
                goal['origins'][0]['creature'] = 'oteryn:creature.rat'
            else:
                goal['origins'][0]['x'] += 1
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                self.document(changed)

    def test_duplicate_cohort_and_goal_bound_refuse(self):
        changed = copy.deepcopy(self.cohort)
        changed['goals'].append(copy.deepcopy(changed['goals'][0]))
        with self.assertRaisesRegex(ValueError, 'DUPLICATE'):
            self.document(changed)
        with self.assertRaisesRegex(ValueError, 'BOUND'):
            self.document({'schema': 1, 'goals': []})


class FibulaNativeGoalProof(unittest.TestCase):
    def test_committed_goal_reproduces_exact_native_origins_and_chosen_cap(self):
        expected = native.document()
        self.assertEqual(json.loads(native.OUTPUT.read_text()), expected)
        goal, = [g for g in expected['goals'] if g['key'].endswith('/fibula-cavern')]
        self.assertEqual(goal['count'], 8)
        self.assertEqual(len(goal['origins']), 21)
        self.assertEqual({p['creature'] for p in goal['origins']},
                         {'oteryn:creature.dragon', 'oteryn:creature.dragon_lord'})
        self.assertEqual(len({(p['x'], p['y'], p['floor'], p['creature'])
                              for p in goal['origins']}), 21)

if __name__ == '__main__':
    unittest.main()
