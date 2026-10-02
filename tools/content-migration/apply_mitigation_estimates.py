"""Install accepted Oteryn balance estimates into a new prepared population.

Only absent mitigation is filled. Existing values and all other gameplay data
are preserved. Acceptance is separate from DERIVED evidence and Global parity.
"""
import argparse
import copy
import hashlib
import json
from fractions import Fraction
from pathlib import Path
import shutil

import complete_creature_dependencies as population
import classify_monster_population as classification

QUALIFICATION = 'OWNER_ACCEPTED_NON_GLOBAL_ESTIMATE'
ACCEPTANCE = 'Owner instruction 2026-10-02: accept estimated missing mitigation as Oteryn balance, without Global parity.'


def read(path):
    return json.loads(Path(path).read_text())


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def patch(documents, estimate, ledger_sha, entry_number):
    documents = copy.deepcopy(documents)
    monster, _, _, manifest = documents
    creature = monster['creature']
    if creature['identity'] != {k: estimate['identity'][k] for k in ('key', 'revision')}:
        raise ValueError('estimate identity mismatch')
    if 'mitigation_percent' in creature['stats']:
        raise ValueError('estimate would replace an existing value')
    if estimate['qualification'] != QUALIFICATION or estimate['evidence_kind'] != 'DERIVED':
        raise ValueError('estimate lacks accepted derived qualification')
    value = estimate['value_ratio']
    ratio = Fraction(value['numerator'], value['denominator'])
    if not 0 <= ratio <= 100 or ratio != Fraction(str(estimate['value_percent'])):
        raise ValueError('invalid or inconsistent mitigation value')
    creature['stats']['mitigation_percent'] = value
    manifest['sources'].append({'kind': 'oteryn_balance_estimate', 'ledger_sha256': ledger_sha,
                               'qualification': QUALIFICATION, 'evidence_classification': 'DERIVED',
                               'global_parity': False, 'owner_acceptance': ACCEPTANCE})
    manifest['entries'].append({'source_index': len(manifest['sources']) - 1,
                               'source_file': 'mitigation-estimates.json', 'source_line': entry_number,
                               'source_field': 'entries.' + estimate['monster'] + '.value_ratio',
                               'kind': 'field', 'status': 'mapped',
                               'destination': '/monster/creature/stats/mitigation_percent',
                               'resolution': QUALIFICATION + '; DERIVED; Global parity unverified. Method: '
                                             + str(estimate['method']) + '; policy: ' + str(estimate['policy'])})
    errors = population.validator.validate(*documents)
    if errors:
        raise ValueError('invalid estimated bundle: ' + str(errors))
    return documents


def apply(baseline, ledger_path, classification_path, annotations_path, output):
    baseline, output = baseline.resolve(), output.resolve()
    protected = [baseline, population.ROOT, ledger_path.resolve().parent]
    if any(output == p or p in output.parents for p in protected):
        raise ValueError('output would modify source inputs')
    if output.exists():
        raise ValueError('output must be new')
    ledger, catalogue = read(ledger_path), read(classification_path)
    index_path = baseline / 'population-index.json'
    classification.validate(catalogue, index_path, baseline / 'bundles')
    if ledger['input_sha256']['population_index'] != sha(index_path) or ledger['input_sha256']['classification'] != sha(classification_path):
        raise ValueError('estimate ledger belongs to another baseline')
    expected = {e['monster'] for e in catalogue['entries'] if e['mitigation']['status'] == 'unknown'}
    estimates = ledger['entries']
    if len({e['monster'] for e in estimates}) != len(estimates) or {e['monster'] for e in estimates} != expected:
        raise ValueError('estimates must cover precisely all missing mitigation')
    rows = {r['monster']: r for r in read(index_path)['monsters']}
    output.mkdir(parents=True)
    patches = output / 'patched-bundles'
    supplements = []
    value_lines = [n for n, line in enumerate(ledger_path.read_text().splitlines(), 1) if '"value_ratio": {' in line]
    if len(value_lines) != len(estimates):
        raise ValueError('ledger requires pretty-printed unique value_ratio lines for provenance')
    for number, estimate in zip(value_lines, estimates):
        name = estimate['monster']
        if Path(name).name != name or name in ('', '.', '..'):
            raise ValueError('invalid monster name')
        source = baseline / 'bundles' / name
        if population.admission.bundle_digest(source) != rows[name]['sha256']:
            raise ValueError('baseline bundle drift')
        values = patch([read(source / f) for f in population.FILES], estimate, sha(ledger_path), number)
        for filename, value in zip(population.FILES, values):
            write(patches / name / filename, value)
        flags = set(rows[name].get('completion_flags', [])) | {QUALIFICATION}
        if estimate['out_of_distribution']:
            flags.add('MITIGATION_ESTIMATE_OUT_OF_DISTRIBUTION')
        row = {**rows[name], 'sha256': population.admission.bundle_digest(patches / name), 'completion_flags': sorted(flags)}
        supplements.append((patches / name, row))
    index = population.merge_population(read(index_path), baseline / 'bundles', output, supplements)
    index['mitigation_balance'] = {'qualification': QUALIFICATION, 'global_parity': False,
                                   'ledger_sha256': sha(ledger_path), 'owner_acceptance': ACCEPTANCE}
    write(output / 'population-index.json', index)
    shutil.copytree(baseline / 'encounters', output / 'encounters')
    annotations = read(annotations_path)
    if annotations['population_index_sha256'] != sha(index_path):
        raise ValueError('annotations belong to another baseline')
    annotations['population_index_sha256'] = sha(output / 'population-index.json')
    write(output / 'classification-source-evidence.json', annotations)
    updated = classification.build(output / 'population-index.json', output / 'bundles', output / 'classification-source-evidence.json')
    for e in updated['entries']:
        if e['monster'] in expected:
            e['mitigation']['reason'] = QUALIFICATION + '; DERIVED Oteryn balance. Global parity unverified; calculation bound by manifest ledger SHA.'
    classification.validate(updated, output / 'population-index.json', output / 'bundles')
    write(output / 'monster-classification.json', updated)
    write(output / 'completion-quality.json', {'schema': 'OTERYN_MITIGATION_BALANCE_COMPLETION/v1',
          'qualification': QUALIFICATION, 'evidence_classification': 'DERIVED', 'global_parity': False,
          'owner_acceptance': ACCEPTANCE, 'ledger_sha256': sha(ledger_path),
          'baseline_index_sha256': sha(index_path), 'estimated_count': len(estimates),
          'unchanged_existing_values': len(rows) - len(estimates),
          'baseline_quality': read(baseline / 'completion-quality.json'),
          'live_gameplay_verified': False, 'production_activated': False})
    return {'prepared': len(rows), 'estimated': len(estimates), 'mitigation': updated['counts']['mitigation']}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for arg in ('baseline', 'ledger', 'classification', 'annotations', 'output'):
        parser.add_argument('--' + arg, required=True, type=Path)
    args = parser.parse_args()
    print(json.dumps(apply(args.baseline, args.ledger, args.classification, args.annotations, args.output)))


if __name__ == '__main__':
    main()
