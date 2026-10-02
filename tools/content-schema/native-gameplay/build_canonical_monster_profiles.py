#!/usr/bin/env python3
"""Append existing canonical Creature policies to a NEW source-attested candidate.

Existing active identities win. No identities, bounds, spawns or donor precedence
are allocated by this export; unsupported execution remains explicitly disabled.
"""
import argparse
import copy
import hashlib
import json
import subprocess
from pathlib import Path
from build_monster_melee_profiles import build as derive_melee, qualify_melee_skill_engine
import build_spell_appearances as appearance

ROOT = Path(__file__).resolve().parents[3]
MAX_BYTES = 8 * 1024 * 1024
MAX_RECORDS = 4096


def identity(value):
    return tuple(value[k] for k in ('family', 'key', 'revision'))


def references(value):
    if isinstance(value, dict):
        if all(k in value for k in ('family', 'key', 'revision')):
            yield identity(value)
        for child in value.values():
            yield from references(child)
    elif isinstance(value, list):
        for child in value:
            yield from references(child)


def compact(value):
    return (json.dumps(value, ensure_ascii=False, separators=(',', ':')) + '\n').encode()


def read_canonical(path):
    raw = path.read_bytes()
    accepted = subprocess.run(['git', 'show', f'HEAD:{path.relative_to(ROOT)}'],
                              cwd=ROOT, check=True, capture_output=True).stdout
    if raw != accepted:
        raise ValueError(f'canonical input changed from HEAD: {path}')
    return json.loads(raw), {'path': str(path.relative_to(ROOT)),
                             'sha256': hashlib.sha256(raw).hexdigest()}


def union_profiles(active, presentations, canonical_rows, declarations):
    if active.get('schema') != 'OTERYN_NATIVE_CREATURE_PROFILES/v1' or presentations.get('schema') != 'OTERYN_NATIVE_PRESENTATION_PROFILES/v1':
        raise ValueError('native profile schema required')
    result = copy.deepcopy(active)
    output_presentations = copy.deepcopy(presentations)
    keys = {r['profile']['target']['key'] for r in result['records']}
    if len(keys) != len(result['records']):
        raise ValueError('duplicate active Creature key')
    profiles = {identity(p['target']): p for p in declarations}
    present = {identity(p['target']): p for p in output_presentations['records']}
    fold_name = lambda value: value.translate(str.maketrans('ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'))
    names = {fold_name(r['profile']['data']['profile']['details']['display_name']): r['profile']['target']
             for r in result['records']}
    if len(names) != len(result['records']):
        raise ValueError('duplicate active Creature display name')
    appended = []
    for row in canonical_rows:
        definition = row['definition']
        target = definition['identity']
        if target['key'] in keys:
            continue
        name = fold_name(row['authoring']['profile']['details']['display_name'])
        if name in names:
            # Native name resolution forbids aliases. Keep the serving actor's
            # exact identity/name; retain the excluded source row in proof.
            continue
        behavior = profiles[identity(definition['behavior'])]
        presentation = profiles[identity(definition['presentation'])]
        result['records'].append({'profile': {'target': copy.deepcopy(target),
                                              'data': copy.deepcopy(row['authoring'])},
                                  'presentation': copy.deepcopy(definition['presentation']),
                                  'behavior': copy.deepcopy(behavior)})
        pref = identity(presentation['target'])
        if pref in present and present[pref] != presentation:
            raise ValueError('same exact Presentation identity has conflicting authoring')
        if pref not in present:
            output_presentations['records'].append(copy.deepcopy(presentation))
            present[pref] = presentation
        keys.add(target['key'])
        names[name] = target
        appended.append(target)
    if len(result['records']) > MAX_RECORDS or len(present) > MAX_RECORDS:
        raise ValueError('native record bound exceeded')
    return result, output_presentations, appended



def extend_illusion_appearances(creatures, presentations, canonical_rows, active, source):
    # Existing source-qualified rows and fixed Avatar selections stay exact.
    result = copy.deepcopy(active)
    if result.get('schema') != 'OTERYN_NATIVE_SPELL_APPEARANCES/v1':
        raise ValueError('native spell appearance schema required')
    if appearance.git(source, 'rev-parse', 'HEAD').decode().strip() != appearance.PIN:
        raise ValueError('Canary source revision mismatch')
    _, _, header_sha = appearance.pinned(source, appearance.HEADER)
    if header_sha != appearance.DEFAULT_SHA or result['default_source_sha256'] != header_sha:
        raise ValueError('source Outfit default closure differs')
    linked = {identity(r['creature']) for r in result['records'] if r['creature'] is not None}
    profiles = {identity(r['target']): r['data']['profile'] for r in presentations['records']}
    canonical = {r['definition']['identity']['key']: r for r in canonical_rows}
    added = []
    for creature in creatures['records']:
        target = creature['profile']['target']
        if not creature['profile']['data']['profile']['details']['flags']['illusionable'] or identity(target) in linked:
            continue
        row = canonical[target['key']]
        bindings = row['source_bindings']
        if len(bindings) != 1 or bindings[0]['disposition'] != 'EXACT' or bindings[0]['target'] != target:
            raise ValueError('missing exact canonical Creature source binding')
        binding = bindings[0]
        if binding['source_key'] != 'oteryn:source.canary' or binding['identity_namespace'] != 'canary/monster-file':
            raise ValueError('additional illusion requires a separately qualified source owner')
        path = 'data-otservbr-global/monster/' + binding['external_id'] + '.lua'
        qualified = appearance.record(source, path, target)
        expected = profiles[identity(creature['presentation'])].get('asset_binding')
        if expected != 'canary.appearance:outfit/' + str(qualified['look_type']):
            raise ValueError('actual source outfit differs from exact canonical Presentation')
        # record() verifies full pinned Git blob bytes, static literal values,
        # bounds, defaults and unsupported mount/object refusal. No flag changes.
        result['records'].append(qualified)
        linked.add(identity(target))
        added.append({'creature': target, 'source_path': path,
                      'source_sha256': qualified['source']['sha256'],
                      'qualification_sha256': qualified['qualification_sha256']})
    return result, added

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--creatures', required=True, type=Path)
    parser.add_argument('--presentations', required=True, type=Path)
    parser.add_argument('--spell-appearances', required=True, type=Path)
    parser.add_argument('--canary-source', required=True, type=Path)
    parser.add_argument('--bundle-root', required=True, type=Path)
    parser.add_argument('--source-root', required=True, type=Path)
    parser.add_argument('--out', required=True, type=Path)
    args = parser.parse_args()
    if args.out.exists():
        parser.error('output directory exists; preserve previously qualified candidates')
    rows, pins = [], []
    for path in sorted((ROOT / 'content/creatures/definitions').glob('creatures-*.json')):
        doc, pin = read_canonical(path)
        rows.extend(doc['records'])
        pins.append(pin)
    declarations, pin = read_canonical(ROOT / 'content/world/definitions/declarations.json')
    pins.append(pin)
    reference, pin = read_canonical(ROOT / 'content/world/definitions/reference.json')
    pins.append(pin)
    active = json.loads(args.creatures.read_bytes())
    presentations = json.loads(args.presentations.read_bytes())
    creatures, presentations, appended = union_profiles(active, presentations, rows,
                                                        declarations['authoring_profiles'])
    selected_keys = {r['profile']['target']['key'] for r in creatures['records']}
    omitted = [{'reason': 'disabled_duplicate_native_display_name', 'canonical_row': r}
               for r in rows if r['definition']['identity']['key'] not in selected_keys]
    creatures, melee = derive_melee(creatures, args.bundle_root, args.source_root,
                                    'canary/data-otservbr-global/monster',
                                    melee_skill_engine=qualify_melee_skill_engine(args.canary_source),
                                    enable_secondary_conditions=True)
    accepted = {identity(r['identity']) for r in reference['records']}
    required = set(references([creatures, presentations]))
    prior_refs = set(references([active, json.loads(args.presentations.read_bytes())]))
    missing_new = sorted((required - accepted) - prior_refs)
    # Unimplemented encounter callbacks remain retained in details, but are
    # never scheduled by the selected melee owner. Any other new dangling ref
    # is an export refusal, not permission to fabricate a definition.
    if any(r[0] != 'Encounter' for r in missing_new):
        raise ValueError(f'new non-Encounter reference absent from accepted registry: {missing_new}')
    spell_appearances, added_illusions = extend_illusion_appearances(
        creatures, presentations, rows, json.loads(args.spell_appearances.read_bytes()), args.canary_source)
    documents = {'creature-profiles.json': creatures, 'presentation-profiles.json': presentations,
                 'spell-appearances.json': spell_appearances}
    encoded = {name: compact(doc) for name, doc in documents.items()}
    if any(len(raw) > MAX_BYTES for raw in encoded.values()):
        raise ValueError('native profile byte bound exceeded')
    proof = {
        'schema': 'OTERYN_CANONICAL_MONSTER_PROFILE_UNION/v1',
        'canonical_inputs': pins,
        'preserved_input_sha256': {
            'creature_profiles': hashlib.sha256(args.creatures.read_bytes()).hexdigest(),
            'presentation_profiles': hashlib.sha256(args.presentations.read_bytes()).hexdigest(),
            'spell_appearances': hashlib.sha256(args.spell_appearances.read_bytes()).hexdigest()},
        'canonical_creature_count': len(rows),
        'preserved_active_count': len(active['records']), 'appended_count': len(appended),
        'creature_count': len(creatures['records']), 'presentation_count': len(presentations['records']),
        'existing_identity_precedence': 'preserve_active_exact_profile',
        'excluded_canonical_records': omitted,
        'identity_allocation': False, 'runtime_activation': False,
        'accepted_reference_count': len(required & accepted),
        'retained_prior_reference_count': len((required - accepted) & prior_refs),
        'disabled_source_only_encounter_references': [dict(zip(('family', 'key', 'revision'), r)) for r in missing_new],
        'melee_enabled': melee['enabled'], 'melee_disabled': melee['disabled'],
        'additional_source_qualified_illusion_appearances': added_illusions,
        'outputs': {name: {'sha256': hashlib.sha256(raw).hexdigest(), 'bytes': len(raw)}
                    for name, raw in encoded.items()},
        'limitations': melee['limitations'],
    }
    args.out.mkdir(parents=True, exist_ok=False)
    for name, raw in encoded.items():
        (args.out / name).write_bytes(raw)
    (args.out / 'monster-melee-derivation.json').write_bytes(compact(melee))
    (args.out / 'canonical-profile-union-proof.json').write_bytes(compact(proof))
    print(json.dumps({k: proof[k] for k in ('creature_count', 'presentation_count', 'appended_count',
                                           'melee_enabled', 'melee_disabled')}))


if __name__ == '__main__':
    main()
