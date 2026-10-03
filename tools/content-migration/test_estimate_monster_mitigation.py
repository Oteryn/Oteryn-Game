import copy
import hashlib
import json
import math
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import estimate_monster_mitigation as model


class MitigationEstimateTests(unittest.TestCase):
    def row(self, name, hp, value=1, display=None, relations=None):
        return {'monster': name, 'identity': {'key': name}, 'display_name': display or name,
                'x': model.features({'max_health': hp, 'armor': hp / 10, 'defense': hp / 5, 'experience': hp}),
                'y': value, 'variant_relations': relations or [], 'confirmed_boss': True}

    def test_grouping_blocks_stat_name_and_transform_leakage(self):
        rows = [self.row('a', 100), self.row('b', 100), self.row('c', 200, display='A'),
                self.row('d', 300, relations=[{'related_identity': {'key': 'c'}}]), self.row('e', 400)]
        model.groups(rows)
        self.assertEqual(1, len({r['group'] for r in rows[:4]}))
        self.assertEqual(1, len({r['fold'] for r in rows[:4]}))
        self.assertNotEqual(rows[0]['group'], rows[4]['group'])

    def test_exact_stats_preserve_conflicting_observations(self):
        rows = [self.row('a', 100, 1), self.row('b', 100, 3), self.row('c', 200, 8)]
        target = self.row('target', 100, None)
        near = model.neighbors(target, rows, model.WEIGHTS[0], model.scales(rows))
        self.assertEqual(2, model.prediction(near, 3))
        self.assertEqual([1, 3, 8], [r['y'] for r in rows])

    def test_prediction_stays_within_training_range_and_is_deterministic(self):
        rows = [self.row('a', 100, 0), self.row('b', 200, 2), self.row('c', 300, 8.16)]
        for hp in (0, 150, 1000000000):
            near = model.neighbors(self.row('target', hp, None), rows, model.WEIGHTS[0], model.scales(rows))
            value = model.prediction(near, 3)
            self.assertTrue(math.isfinite(value) and 0 <= value <= 8.16)
            self.assertEqual(value, model.prediction(copy.deepcopy(near), 3))

    def test_cross_validation_predicts_every_grouped_row_once(self):
        rows = [self.row(str(i), i * 100 + 1, i / 10) for i in range(40)]
        model.groups(rows)
        results = model.cross_validate(rows, 'global')
        self.assertEqual(16, len(results))
        self.assertTrue(all(r['n'] == 40 for r in results))
        self.assertEqual(model.choose(results), model.choose(list(reversed(results))))

    def test_explicit_owner_balance_floor_and_special_cap(self):
        self.assertEqual((0.01, 8.16), model.accepted_value(0, 8.16, False))
        self.assertEqual((1.0, 1.0), model.accepted_value(7.5, 8.16, True))
        self.assertEqual((2.76, 8.16), model.accepted_value(2.764, 8.16, False))

    def test_mutated_bundle_is_rejected_before_estimation(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            folder = root / 'bundles' / 'a'
            folder.mkdir(parents=True)
            identity = {'key': 'a', 'revision': 'r1'}
            creature = {'identity': identity, 'stats': {'max_health': 100, 'armor': 10, 'defense': 20, 'experience': 100}}
            (folder / 'monster.json').write_text(json.dumps({'creature': creature}))
            digest = hashlib.sha256()
            for filename in ('monster.json', 'dependencies.json', 'catalog.json', 'manifest.json'):
                path = folder / filename
                if not path.exists():
                    path.write_text('{}')
                data = path.read_bytes()
                digest.update(f'{filename}\0{len(data)}\0'.encode('ascii') + data)
            index = root / 'population-index.json'
            index.write_text(json.dumps({'monsters': [{'monster': 'a', 'sha256': digest.hexdigest()}]}))
            catalog = root / 'classification.json'
            catalog.write_text(json.dumps({'population_index_sha256': model.sha(index), 'entries': [{
                'monster': 'a', 'identity': identity, 'bundle_sha256': digest.hexdigest(),
                'display_name': 'a', 'variant_relations': [], 'roles': [], 'mitigation': {'status': 'unknown'}}]}))
            self.assertEqual(1, len(model.load_rows(root, catalog)[0]))
            creature['stats']['armor'] = 99
            (folder / 'monster.json').write_text(json.dumps({'creature': creature}))
            with self.assertRaisesRegex(ValueError, 'Bundle bytes differ'):
                model.load_rows(root, catalog)

    def test_invalid_features_rejected(self):
        for value in (-1, float('nan'), float('inf'), True):
            with self.assertRaises(ValueError):
                model.features({'max_health': value, 'armor': 0, 'defense': 0, 'experience': 0})

    def test_accepted_estimates_never_train_or_change_source_model(self):
        observed = [dict(self.row(str(i), i * 100 + 1, i / 10),
                         estimate_markers=[], boss=True, special=False) for i in range(40)]
        target = dict(self.row('target', 1500, None), estimate_markers=[], boss=False, special=False)
        hashes = {'population_index_sha256': 'index', 'classification_sha256': 'catalog'}
        with patch.object(model, 'load_rows', return_value=(copy.deepcopy(observed + [target]), hashes)):
            before = model.build('unused', 'unused')
        accepted = dict(self.row('accepted', 1500, 999),
                        estimate_markers=['population_completion_flag'], boss=True, special=False)
        with patch.object(model, 'load_rows', return_value=(copy.deepcopy(observed + [target, accepted]), hashes)):
            after = model.build('unused', 'unused')
        self.assertEqual(40, after['known_count'])
        self.assertEqual(1, after['preserved_estimate_count'])
        self.assertEqual(before['entries'], after['entries'])
        self.assertEqual(before['cross_validation'], after['cross_validation'])
        self.assertEqual(999, after['excluded_estimates'][0]['value_percent'])

    def test_estimate_detection_with_either_independent_provenance_marker(self):
        for marker in ('manifest', 'flag', 'none'):
            with self.subTest(marker=marker), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                folder = root / 'bundles' / 'a'
                folder.mkdir(parents=True)
                identity = {'key': 'a', 'revision': 'r1'}
                ratio = {'numerator': 1, 'denominator': 1}
                creature = {'identity': identity, 'stats': {'max_health': 100, 'armor': 10,
                            'defense': 20, 'experience': 100, 'mitigation_percent': ratio}}
                documents = {'monster.json': {'creature': creature}, 'dependencies.json': {},
                             'catalog.json': {}, 'manifest.json': {'sources':
                             [{'kind': 'oteryn_balance_estimate'}] if marker == 'manifest' else []}}
                digest = hashlib.sha256()
                for filename, document in documents.items():
                    data = json.dumps(document).encode()
                    (folder / filename).write_bytes(data)
                    digest.update(f'{filename}\0{len(data)}\0'.encode('ascii') + data)
                index = root / 'population-index.json'
                index.write_text(json.dumps({'monsters': [{'monster': 'a', 'sha256': digest.hexdigest(),
                    'completion_flags': [model.QUALIFICATION] if marker == 'flag' else []}]}))
                catalog = root / 'classification.json'
                catalog.write_text(json.dumps({'population_index_sha256': model.sha(index), 'entries': [{
                    'monster': 'a', 'identity': identity, 'bundle_sha256': digest.hexdigest(),
                    'display_name': 'a', 'variant_relations': [], 'roles': [],
                    'mitigation': {'status': 'present', 'value': ratio}}]}))
                row = model.load_rows(root, catalog)[0][0]
                self.assertEqual(1, row['y'])
                self.assertEqual(marker != 'none', bool(row['estimate_markers']))

    def test_no_missing_values_returns_empty_ledger_without_reestimating(self):
        rows = [dict(self.row(str(i), i * 100 + 1, i / 10), estimate_markers=[],
                     boss=True, special=False) for i in range(40)]
        hashes = {'population_index_sha256': 'index', 'classification_sha256': 'catalog'}
        with patch.object(model, 'load_rows', return_value=(rows, hashes)):
            ledger = model.build('unused', 'unused')
        self.assertEqual([], ledger['entries'])
        self.assertIsNone(ledger['summary']['estimated_range_percent'])


if __name__ == '__main__':
    unittest.main()
