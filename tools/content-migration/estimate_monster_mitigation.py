"""Deterministic, owner-accepted Oteryn balance estimates; no Global parity claim.

Read-only population inputs. Grouped five-fold CV keeps identical stat tuples,
names and explicit transformation families together. Errors are percentage points.
"""
import argparse
from collections import Counter
from fractions import Fraction
import hashlib
import json
import math
from pathlib import Path
import re
import statistics

WEIGHTS = ((1, 2, 1, 0), (1, 2, 1, 0.5), (2, 2, 1, 0.5), (1, 1, 1, 1))
KS = (3, 5, 9, 15)
QUALIFICATION = 'OWNER_ACCEPTED_NON_GLOBAL_ESTIMATE'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def features(stats):
    raw = [stats[k] for k in ('max_health', 'armor', 'defense', 'experience')]
    if any(isinstance(v, bool) or not isinstance(v, (int, float)) or not math.isfinite(v) or v < 0 for v in raw):
        raise ValueError('Invalid numeric statistics')
    return (math.log1p(raw[0]), raw[1], raw[2], math.log1p(raw[3]))


def groups(rows):
    parent = list(range(len(rows)))
    def find(i):
        while parent[i] != i:
            parent[i] = parent[parent[i]]
            i = parent[i]
        return i
    def union(i, j):
        a, b = find(i), find(j)
        parent[max(a, b)] = min(a, b)
    seen, identities = {}, {r['identity']['key']: i for i, r in enumerate(rows)}
    for i, row in enumerate(rows):
        name = re.sub(r'[^a-z0-9]', '', row['display_name'].lower())
        for key in [('stats', row['x']), ('name', name)]:
            if key in seen:
                union(i, seen[key])
            seen[key] = i
        for relation in row['variant_relations']:
            other = identities.get(relation['related_identity']['key'])
            if other is not None:
                union(i, other)
    for i, row in enumerate(rows):
        row['group'] = rows[find(i)]['monster']
        row['fold'] = int(hashlib.sha256(row['group'].encode()).hexdigest(), 16) % 5


def scales(training):
    return tuple(statistics.pstdev([r['x'][i] for r in training]) or 1 for i in range(4))


def neighbors(target, training, weight, scale):
    return sorted(((sum(w * ((a - b) / s) ** 2 for a, b, s, w in zip(target['x'], r['x'], scale, weight)), r)
                   for r in training), key=lambda pair: (pair[0], pair[1]['monster']))


def prediction(near, k):
    selected = near[:k]
    # Exact-stat neighbors agree on distance, but can legitimately disagree on mitigation.
    exact = [r['y'] for d, r in near if d < 1e-15]
    if exact:
        return statistics.mean(exact)
    weights = [1 / (math.sqrt(d) + 0.1) for d, _ in selected]
    return sum(w * r['y'] for w, (_, r) in zip(weights, selected)) / sum(weights)


def accepted_value(raw_prediction, maximum, special):
    ceiling = min(1.0, maximum) if special else maximum
    return round(max(0.01, min(ceiling, raw_prediction)), 2), ceiling


def metrics(errors):
    errors = sorted(errors)
    return {'n': len(errors), 'mae_pp': round(statistics.mean(errors), 6),
            'p90_pp': round(errors[math.ceil(len(errors) * 0.9) - 1], 6)}


def cross_validate(known, scope):
    reports = []
    for weight in WEIGHTS:
        residuals = {k: [] for k in KS}
        for fold in range(5):
            train = [r for r in known if r['fold'] != fold and (scope == 'global' or r['confirmed_boss'])]
            targets = [r for r in known if r['fold'] == fold and (scope == 'global' or r['confirmed_boss'])]
            if not train or not targets:
                continue
            scale = scales(train)
            for row in targets:
                near = neighbors(row, train, weight, scale)
                for k in KS:
                    residuals[k].append((row, abs(prediction(near, k) - row['y'])))
        for k in KS:
            errors = [e for _, e in residuals[k]]
            boss_errors = [e for r, e in residuals[k] if r['confirmed_boss']]
            reports.append({'scope': scope, 'k': k, 'weights': list(weight), **metrics(errors),
                            'boss_validation': metrics(boss_errors)})
    return reports


def choose(reports, boss=False):
    def score(r):
        m = r['boss_validation'] if boss else r
        return m['mae_pp'] + 0.2 * m['p90_pp']
    best = min(score(r) for r in reports)
    # Within 2% of best, prefer a modest smoother neighborhood and no XP reliance.
    eligible = [r for r in reports if score(r) <= best * 1.02]
    return min(eligible, key=lambda r: (abs(r['k'] - 5), r['weights'][3], score(r), tuple(r['weights'])))


def load_rows(population, classification):
    population, classification = Path(population), Path(classification)
    index_path = population / 'population-index.json'
    index, catalog = json.loads(index_path.read_text()), json.loads(classification.read_text())
    if catalog['population_index_sha256'] != sha(index_path):
        raise ValueError('Classification belongs to another population')
    by_name = {r['monster']: r for r in index['monsters']}
    entries = catalog['entries']
    if len(by_name) != len(entries) or set(by_name) != {r['monster'] for r in entries}:
        raise ValueError('Classification coverage differs')
    rows, input_hashes = [], {}
    for entry in sorted(entries, key=lambda r: r['monster']):
        name = entry['monster']
        if Path(name).name != name or entry['bundle_sha256'] != by_name[name]['sha256']:
            raise ValueError('Invalid or stale classified bundle')
        digest = hashlib.sha256()
        for filename in ('monster.json', 'dependencies.json', 'catalog.json', 'manifest.json'):
            data = (population / 'bundles' / name / filename).read_bytes()
            digest.update(f'{filename}\0{len(data)}\0'.encode('ascii') + data)
        if digest.hexdigest() != entry['bundle_sha256']:
            raise ValueError('Bundle bytes differ from source-bound snapshot')
        path = population / 'bundles' / name / 'monster.json'
        creature = json.loads(path.read_text())['creature']
        if any(creature['identity'][k] != entry['identity'][k] for k in ('key', 'revision')):
            raise ValueError('Creature identity mismatch')
        ratio = creature['stats'].get('mitigation_percent')
        y = float(Fraction(ratio['numerator'], ratio['denominator'])) if ratio is not None else None
        if (y is None) != (entry['mitigation']['status'] == 'unknown'):
            raise ValueError('Mitigation status mismatch')
        if y is not None and entry['mitigation']['value'] != ratio:
            raise ValueError('Classified mitigation differs from source value')
        if y is not None and (not math.isfinite(y) or y < 0):
            raise ValueError('Invalid observed mitigation')
        role_names = {c['role'] for c in entry['roles']}
        rows.append({**entry, 'x': features(creature['stats']), 'y': y,
                     'confirmed_boss': any(c['role'] == 'boss' and c['confidence'] == 'confirmed' for c in entry['roles']),
                     'boss': 'boss' in role_names, 'special': bool(role_names & {'mechanic_actor', 'trainer', 'familiar'})})
        input_hashes[f'bundles/{name}/monster.json'] = sha(path)
    groups(rows)
    return rows, {'population_index_sha256': sha(index_path), 'classification_sha256': sha(classification),
                  'monster_input_sha256': input_hashes}


def build(population, classification):
    rows, hashes = load_rows(population, classification)
    known = [r for r in rows if r['y'] is not None]
    missing = [r for r in rows if r['y'] is None]
    global_reports, boss_reports = cross_validate(known, 'global'), cross_validate(known, 'boss')
    global_model = choose(global_reports)
    boss_model = choose([global_model] + boss_reports, boss=True)
    maximum = max(r['y'] for r in known)
    entries = []
    for row in missing:
        model = boss_model if row['boss'] and not row['special'] else global_model
        training = [r for r in known if model['scope'] == 'global' or r['confirmed_boss']]
        scale = scales(training)
        near = neighbors(row, training, model['weights'], scale)
        raw_prediction = prediction(near, model['k'])
        value, ceiling = accepted_value(raw_prediction, maximum, row['special'])
        outside = [i for i in range(4) if row['x'][i] < min(r['x'][i] for r in training) or row['x'][i] > max(r['x'][i] for r in training)]
        # HP of a mechanic is not a verified combat-strength signal. No separate extrapolation.
        uncertainty = (model['boss_validation'] if model['scope'] == 'boss' else model)['p90_pp']
        uncertainty = max(uncertainty, statistics.pstdev([r['y'] for _, r in near[:model['k']]]))
        if outside or row['special']:
            uncertainty *= 1.5
        fraction = Fraction(str(value))
        entries.append({'monster': row['monster'], 'identity': row['identity'], 'value_percent': value,
                        'value_ratio': {'numerator': fraction.numerator, 'denominator': fraction.denominator},
                        'qualification': QUALIFICATION, 'evidence_kind': 'DERIVED', 'global_parity': False,
                        'balance_floor_percent': 0.01, 'balance_ceiling_percent': ceiling,
                        'raw_prediction_percent': round(raw_prediction, 6),
                        'policy': 'SPECIAL_CONSERVATIVE_INTERPOLATION' if row['special'] else 'BOSS_KNN' if model['scope'] == 'boss' else 'GLOBAL_KNN',
                        'method': {'scope': model['scope'], 'k': model['k'], 'weights': model['weights']},
                        'input_features': dict(zip(('log1p_hp', 'armor', 'defense', 'log1p_xp'), row['x'])),
                        'out_of_distribution': bool(outside), 'outside_feature_indices': outside,
                        'uncertainty_interval_percent': [round(max(0, value - uncertainty), 2), round(min(maximum, value + uncertainty), 2)],
                        'uncertainty_qualification': 'Heuristic residual/spread range; not a calibrated confidence interval.',
                        'nearest_neighbors': [{'monster': r['monster'], 'identity': r['identity'], 'value_percent': r['y'],
                                               'distance': round(math.sqrt(d), 6)} for d, r in near[:model['k']]]})
    return {'schema': 'OTERYN_ACCEPTED_MITIGATION_ESTIMATES/v1', 'qualification': QUALIFICATION,
            'global_parity': False, 'input_hashes': hashes,
            'input_sha256': {'population_index': hashes['population_index_sha256'], 'classification': hashes['classification_sha256']}, 'known_count': len(known), 'estimated_count': len(entries),
            'confirmed_boss_training_count': sum(r['confirmed_boss'] for r in known),
            'grouping': 'Union of identical stat tuples, normalized display names and explicit transformation relations; SHA256 group modulo 5.',
            'selection': 'MAE + 0.2*p90; within 2%, prefer k near 5, then lower XP weight.',
            'balance_policy': 'Missing values have explicit Oteryn floor 0.01%; mechanic/familiar/trainer estimates capped at 1%. Existing values unchanged. These policy choices are not Global facts.',
            'selected_global_model': global_model, 'selected_boss_model': boss_model,
            'cross_validation': global_reports + boss_reports, 'observed_range_percent': [min(r['y'] for r in known), maximum],
            'summary': {'policy': dict(Counter(e['policy'] for e in entries)), 'ood_count': sum(e['out_of_distribution'] for e in entries),
                        'estimated_range_percent': [min(e['value_percent'] for e in entries), max(e['value_percent'] for e in entries)]},
            'entries': entries}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--population', type=Path, required=True)
    parser.add_argument('--classification', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    if args.out.exists():
        raise ValueError('Output must be a new directory')
    ledger = build(args.population, args.classification)
    args.out.mkdir(parents=True)
    (args.out / 'mitigation-estimates.json').write_text(json.dumps(ledger, indent=2) + '\n')
    print(json.dumps({k: ledger[k] for k in ('known_count', 'estimated_count', 'selected_global_model', 'selected_boss_model', 'summary')}, indent=2))


if __name__ == '__main__':
    main()
