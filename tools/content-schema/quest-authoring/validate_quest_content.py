"""Validate reward claims, reward-only quests and door gates: JSON Schema plus the semantic rules of the format.

Usage: python validate_quest_content.py CLAIMS.json QUESTS.json [--catalog CATALOG.json] [--manifest MANIFEST.json]
                                        [--gates GATES.json [--gates-manifest MANIFEST.json]]
Prints one JSON report; the exit code is 1 when the documents are invalid.
"""
import argparse
import json
import sys
from pathlib import Path

import jsonschema

ROOT = Path(__file__).resolve().parent
MANIFEST_STATUS = ('mapped', 'conflict', 'approved_omission', 'unresolved_semantics')


def refs(value):
    if isinstance(value, dict):
        if set(value) == {'family', 'key', 'revision'}:
            yield value
            return
        for child in value.values():
            yield from refs(child)
    elif isinstance(value, list):
        for child in value:
            yield from refs(child)


def validate(claims_doc, quests_doc, catalog=None, manifest=None):
    schema = json.loads((ROOT / 'quest_content.schema.json').read_text())
    validator = jsonschema.Draft202012Validator(schema)
    errors = [f'{"/".join(map(str, e.absolute_path))}: {e.message}'
              for doc in (claims_doc, quests_doc) for e in validator.iter_errors(doc)]
    if errors:
        return errors
    claims, quests = claims_doc['claims'], quests_doc['quests']
    claim_keys, quest_keys, positions = set(), set(), {}
    for claim in claims:
        key = claim['identity']['key']
        if key in claim_keys:
            errors.append(f'{key}: duplicate claim key')
        claim_keys.add(key)
        for placement in claim['placements']:
            reward = placement['reward']
            if not reward['items'] and not reward.get('random_one_of'):
                errors.append(f'{key}: a placement hands out nothing')
            position = tuple(placement['position'].values())
            if position in positions:
                errors.append(f'{key}: position {position} already belongs to {positions[position]}')
            positions[position] = key
            text = reward.get('written_text')
            carriers = [q['item'] for q in reward['items']] + ([reward['container']] if reward.get('container') else [])
            if text and text['item'] and text['item'] not in carriers:
                errors.append(f'{key}: written text names an item the chest does not hand out')
        if (claim['quest'] is None) != (claim['quest_link_basis'] is None):
            errors.append(f'{key}: quest and quest_link_basis must be set together')
        if claim['quest'] and claim['quest_candidate_from_section']:
            errors.append(f'{key}: a linked claim has no section candidate')
    paths = {}
    for quest in quests:
        key = quest['identity']['key']
        if key in quest_keys:
            errors.append(f'{key}: duplicate quest key')
        quest_keys.add(key)
        path = key.split(':', 1)[1]
        if paths.setdefault(path, key) != key:
            errors.append(f'{key}: the same quest is also {paths[path]} (one identity per quest across namespaces)')
        for claim_ref in quest['claims']:
            if claim_ref['family'] != 'RewardClaim' or claim_ref['key'] not in claim_keys:
                errors.append(f'{key}: unknown claim {claim_ref["key"]}')
    linked = {(c['identity']['key'], c['quest']['key']) for c in claims if c['quest']}
    listed = {(r['key'], q['identity']['key']) for q in quests for r in q['claims']}
    for claim_key, quest_key in sorted(linked ^ listed):
        errors.append(f'{claim_key}: claim and quest {quest_key} do not point at each other')
    if catalog is not None:
        known = {(d['family'], d['key']) for d in catalog['definitions']}
        for r in refs(claims):
            if r['family'] in ('Item', 'Achievement') and (r['family'], r['key']) not in known:
                errors.append(f'{r["key"]}: {r["family"]} reference missing from the catalog')
    if manifest is not None:
        for entry in manifest['entries']:
            if entry['status'] not in MANIFEST_STATUS:
                errors.append(f'manifest: unknown status {entry["status"]}')
            if entry.get('destination') and entry['destination'] not in claim_keys:
                errors.append(f'manifest: destination {entry["destination"]} is not a claim')
            if entry['status'] in ('mapped', 'conflict') and not entry.get('destination'):
                errors.append(f'manifest: {entry["status"]} entry at {entry.get("position")} has no destination')
        mapped = {e['destination'] for e in manifest['entries'] if e.get('destination')}
        for key in sorted(claim_keys - mapped):
            errors.append(f'{key}: no manifest entry maps a source chest to this claim')
    return sorted(set(errors))


def validate_gates(gates_doc, claims_doc, manifest=None):
    schema = json.loads((ROOT / 'quest_content.schema.json').read_text())
    errors = [f'{"/".join(map(str, e.absolute_path))}: {e.message}'
              for e in jsonschema.Draft202012Validator(schema).iter_errors(gates_doc)]
    if errors:
        return errors
    claims = {c['identity']['key']: c for c in claims_doc['claims']}
    keys, positions = set(), {}
    for gate in gates_doc['gates']:
        key, condition = gate['identity']['key'], gate['condition']
        if key in keys:
            errors.append(f'{key}: duplicate gate key')
        keys.add(key)
        if (gate['quest'] is None) != (gate['quest_link_basis'] is None):
            errors.append(f'{key}: quest and quest_link_basis must be set together')
        if (gate['state'] == 'shared_lock') != (condition['kind'] == 'door_key'):
            errors.append(f'{key}: only a key door has a shared lock')
        for placement in gate['placements']:
            position = tuple(placement['position'].values())
            if position in positions:
                errors.append(f'{key}: position {position} already belongs to {positions[position]}')
            positions[position] = key
        if condition['kind'] == 'quest_progress' and condition['claim']:
            claim = claims.get(condition['claim']['key'])
            if claim is None:
                errors.append(f'{key}: unknown claim {condition["claim"]["key"]}')
            elif claim['identity']['key'].split('/', 1)[1] != condition['progress'].split('/', 1)[1]:
                errors.append(f'{key}: the claim does not record the progress the door reads')
        if condition['kind'] == 'door_key':
            number = condition['key_binding'].rsplit('/', 1)[1]
            for claim_ref in condition['key_from_claims']:
                claim = claims.get(claim_ref['key'])
                bindings = {p['reward'].get('key_binding', '').rsplit('/', 1)[-1] for p in claim['placements']} if claim else set()
                if number not in bindings:
                    errors.append(f'{key}: {claim_ref["key"]} hands out no key for this door')
    if manifest is not None:
        for entry in manifest['entries']:
            if entry['status'] not in MANIFEST_STATUS:
                errors.append(f'manifest: unknown status {entry["status"]}')
            if entry.get('destination') and entry['destination'] not in keys:
                errors.append(f'manifest: destination {entry["destination"]} is not a gate')
            if entry['status'] in ('mapped', 'conflict') and not entry.get('destination'):
                errors.append(f'manifest: {entry["status"]} entry at {entry.get("position")} has no destination')
        mapped = {e['destination'] for e in manifest['entries'] if e.get('destination')}
        for key in sorted(keys - mapped):
            errors.append(f'{key}: no manifest entry maps a source door to this gate')
    return sorted(set(errors))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('claims', type=Path)
    parser.add_argument('quests', type=Path)
    parser.add_argument('--catalog', type=Path)
    parser.add_argument('--manifest', type=Path)
    parser.add_argument('--gates', type=Path)
    parser.add_argument('--gates-manifest', type=Path)
    args = parser.parse_args()
    load = lambda p: json.loads(p.read_text()) if p else None
    errors = validate(load(args.claims), load(args.quests), load(args.catalog), load(args.manifest))
    if args.gates:
        errors += validate_gates(load(args.gates), load(args.claims), load(args.gates_manifest))
    print(json.dumps({'valid': not errors, 'errors': errors[:50], 'error_count': len(errors)}, indent=2))
    sys.exit(1 if errors else 0)


if __name__ == '__main__':
    main()
