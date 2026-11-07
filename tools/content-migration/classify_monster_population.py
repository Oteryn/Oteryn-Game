"""Build a source-bound authoring classification catalogue, without changing combat.

Roles and contexts overlap. Missing encyclopedia/stat fields never establish a
role or non-applicability. This catalogue is separate from the closed native
Creature contract and must travel with its matching population snapshot.
"""
import argparse
import copy
from collections import Counter
import hashlib
import json
from pathlib import Path

import jsonschema
import complete_creature_dependencies as population

SCHEMA = Path(__file__).with_name('monster-classification.schema.json')
ROLES = ('creature', 'boss', 'summon', 'familiar', 'trainer', 'mechanic_actor', 'unknown')
CONTEXTS = ('world', 'quest', 'raid', 'event', 'dawnport', 'historical', 'encounter', 'unknown')


def read(path):
    return json.loads(Path(path).read_text())


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def counts(entries):
    roles, contexts, mitigation = Counter(), Counter(), Counter()
    for entry in entries:
        for claim in entry['roles']:
            roles[claim['role'] + ':' + claim['confidence']] += 1
        for context, confidence in {(c['context'], c['confidence']) for c in entry['contexts']}:
            contexts[context + ':' + confidence] += 1
        mitigation[entry['mitigation']['status']] += 1
    return {'population': len(entries), 'roles': dict(sorted(roles.items())),
            'contexts': dict(sorted(contexts.items())), 'mitigation': dict(sorted(mitigation.items()))}


def encounter_inputs(bundles):
    root = bundles.parent
    return {str(p.relative_to(root)): sha(p) for p in sorted((root / 'encounters').rglob('*.json'))}


def check_bundle_evidence(evidence, bundles):
    if evidence['kind'] not in ('bundle_field', 'bundle_summon_reference', 'bundle_field_absent'):
        return
    path = Path(evidence['source'])
    if len(path.parts) != 3 or path.parts[0] != 'bundles' or path.parts[1] in ('.', '..') or path.parts[2] != 'monster.json':
        raise ValueError('bundle evidence has an invalid source path')
    value = read(bundles / path.parts[1] / path.parts[2])
    try:
        for part in evidence['pointer'].split('/')[1:]:
            part = part.replace('~1', '/').replace('~0', '~')
            value = value[int(part)] if isinstance(value, list) else value[part]
    except (KeyError, IndexError):
        if evidence['kind'] == 'bundle_field_absent':
            return
        raise ValueError('positive bundle evidence pointer is absent')
    if evidence['kind'] == 'bundle_field_absent':
        raise ValueError('absent-field evidence hides a present value')
    if 'value' in evidence and value != evidence['value']:
        raise ValueError('bundle evidence value differs from its pointer')


def build(index_path, bundles, annotations_path):
    index, packet = read(index_path), read(annotations_path)
    if packet['population_index_sha256'] != sha(index_path):
        raise ValueError('classification evidence belongs to a different population')
    names = {r['monster'] for r in index['monsters']}
    if names != set(packet['annotations']):
        raise ValueError('classification evidence does not cover the complete population')
    entries = []
    for row in index['monsters']:
        name = row['monster']
        if Path(name).name != name or name in ('', '.', '..'):
            raise ValueError('invalid bundle name')
        folder = bundles / name
        if population.admission.bundle_digest(folder) != row['sha256']:
            raise ValueError('source bundle digest differs: ' + name)
        creature = read(folder / 'monster.json')['creature']
        annotation = packet['annotations'][name]
        mitigation = {'status': 'unknown', 'reason': 'Absent from this prepared record; this does not establish absence in Global.',
                      'evidence': [{'kind': 'bundle_field_absent', 'source': f'bundles/{name}/monster.json',
                                    'pointer': '/creature/stats/mitigation_percent'}]}
        if 'mitigation_percent' in creature['stats']:
            mitigation = {'status': 'present', 'value': creature['stats']['mitigation_percent'],
                          'reason': 'Value is present in the source-bound candidate; presence alone does not prove Global parity.',
                          'evidence': [{'kind': 'bundle_field', 'source': f'bundles/{name}/monster.json',
                                        'pointer': '/creature/stats/mitigation_percent',
                                        'value': creature['stats']['mitigation_percent']}]}
        entries.append({'monster': name, 'identity': {'family': 'Creature', **creature['identity']},
                        'display_name': creature['display_name'], 'bundle_sha256': row['sha256'],
                        'roles': copy.deepcopy(annotation['roles']), 'contexts': copy.deepcopy(annotation['contexts']),
                        'source_files': copy.deepcopy(annotation.get('source_files', [])),
                        'encounter_memberships': copy.deepcopy(annotation['encounter_memberships']),
                        'variant_relations': copy.deepcopy(annotation['variant_relations']),
                        'encyclopedia': {'bestiary': 'bestiary' in creature, 'bosstiary': 'bosstiary' in creature},
                        'mitigation': mitigation})
    result = {'schema': 'OTERYN_MONSTER_CLASSIFICATION/v1', 'authoring_only': True,
              'population_index_sha256': sha(index_path), 'classification_evidence_sha256': sha(annotations_path),
              'encounter_input_sha256': encounter_inputs(bundles),
              'entries': entries, 'counts': counts(entries)}
    validate(result, index_path, bundles)
    return result


def validate(catalogue, index_path, bundles):
    jsonschema.Draft202012Validator(read(SCHEMA)).validate(catalogue)
    index = read(index_path)
    if catalogue['population_index_sha256'] != sha(index_path):
        raise ValueError('classification index SHA is stale')
    expected = {r['monster']: r for r in index['monsters']}
    actual = {r['monster']: r for r in catalogue['entries']}
    if len(actual) != len(catalogue['entries']) or set(actual) != set(expected):
        raise ValueError('classification population is incomplete or duplicated')
    if catalogue['counts'] != counts(catalogue['entries']):
        raise ValueError('classification counts disagree with records')
    if catalogue['encounter_input_sha256'] != encounter_inputs(bundles):
        raise ValueError('classification Encounter inputs are stale')
    identities = {(e['identity']['key'], e['identity']['revision']) for e in catalogue['entries']}
    for name, entry in actual.items():
        folder = bundles / name
        if entry['bundle_sha256'] != expected[name]['sha256'] or population.admission.bundle_digest(folder) != entry['bundle_sha256']:
            raise ValueError('classification bundle SHA is stale: ' + name)
        creature = read(folder / 'monster.json')['creature']
        if entry['identity'] != {'family': 'Creature', **creature['identity']}:
            raise ValueError('classification identity differs from bundle')
        roles = entry['roles']
        if any(r['role'] == 'unknown' for r in roles) and len(roles) != 1:
            raise ValueError('unknown cannot conceal a classified role')
        for claim in roles + entry['contexts']:
            label = claim.get('role', claim.get('context'))
            if (label == 'unknown') != (claim['confidence'] == 'unknown'):
                raise ValueError('unknown labels and confidence must agree')
            if claim['confidence'] != 'unknown' and not claim['evidence']:
                raise ValueError('classification assertion lacks evidence')
            if claim['confidence'] == 'confirmed' and all(e['kind'] == 'source_folder' for e in claim['evidence']):
                raise ValueError('source folder alone cannot confirm a role/context')
            for evidence in claim['evidence']:
                check_bundle_evidence(evidence, bundles)
        mitigation = entry['mitigation']
        for evidence in mitigation['evidence']:
            check_bundle_evidence(evidence, bundles)
        present = 'mitigation_percent' in creature['stats']
        if mitigation['status'] == 'present':
            if not present or mitigation['value'] != creature['stats']['mitigation_percent']:
                raise ValueError('mitigation value differs from bundle')
        elif present:
            raise ValueError('classification hides an existing mitigation value')
        elif mitigation['status'] == 'not_applicable':
            if not any(e['kind'] == 'positive_non_applicability_source'
                       and all(k in e for k in ('repository', 'revision', 'line', 'qualification'))
                       for e in mitigation['evidence']):
                raise ValueError('non-applicability requires positive source evidence')
        for relation in entry['variant_relations']:
            target = relation['related_identity']
            if (target['key'], target['revision']) not in identities:
                raise ValueError('variant relation has an unresolved identity')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('index', 'bundles', 'annotations', 'out'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    if args.out.exists():
        raise ValueError('output must be new')
    if args.out.resolve() == args.bundles.resolve() or args.bundles.resolve() in args.out.resolve().parents:
        raise ValueError('output would modify source bundles')
    result = build(args.index, args.bundles, args.annotations)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(result, indent=2, ensure_ascii=False) + '\n')
    print(json.dumps(result['counts']))


if __name__ == '__main__':
    main()
