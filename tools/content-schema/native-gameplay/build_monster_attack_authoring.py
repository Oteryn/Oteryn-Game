#!/usr/bin/env python3
"""Derive monster attack ability authoring for every `attacks[]` reference of the native creature
profile artifact.

Input is only repository evidence: `spell-native-profiles.json` and the prepared population
archive (per-monster donor `monster.json`, `dependencies.json`, `manifest.json`). Nothing is
invented. A reference whose donor closure is absent, conflicting or unresolvable is written as an
explicit `unavailable` record with a reason; the validator fails on any reference without a record.
"""
from __future__ import annotations

import argparse
import collections
import hashlib
import io
import json
import math
import tarfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PROFILES = 'content/creatures/definitions/spell-native-profiles.json'
ARCHIVE = 'docs/agents/evidence/monster-completion-final-20261002/prepared-population.tar.gz'
OUT_DIR = 'content/abilities/source-imports'
RECORDS = f'{OUT_DIR}/monster-attack-authoring.jsonl'
PIN = f'{OUT_DIR}/monster-attack-authoring.pin.json'
SCHEMA = 'OTERYN_MONSTER_ATTACK_AUTHORING/v1'
PIN_SCHEMA = 'OTERYN_MONSTER_ATTACK_AUTHORING_PIN/v1'
QUALIFICATION = 'DONOR_ATTESTED_NOT_GLOBAL_VERIFIED'
MELEE_SKILL_FORMULA = 'maximum=ceil(skill*(attack*0.05)+attack*0.5); minimum=0'
REPOSITORY_ROOTS = {'opentibiabr/canary': 'data-otservbr-global/', 'zimbadev/crystalserver': 'data-global/'}
BUNDLE_FILES = ('monster.json', 'dependencies.json', 'manifest.json')
# Later directories replace earlier ones for the same bundle name (patched population).
BUNDLE_DIRS = ('population/bundles', 'population/patched-bundles')


def canonical(value) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()


def sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def ref_key(ref):
    return ref['key'], ref['revision']


def load_bundles(archive: Path):
    """Return {bundle name: {file: bytes}} with patched bundles replacing the base ones."""
    found = {}
    with tarfile.open(archive, 'r:gz') as tar:
        for member in tar:
            if not member.isfile():
                continue
            parts = member.name.split('/')
            if len(parts) != 4 or '/'.join(parts[:2]) not in BUNDLE_DIRS or parts[3] not in BUNDLE_FILES:
                continue
            raw = tar.extractfile(member).read()
            found.setdefault((BUNDLE_DIRS.index('/'.join(parts[:2])), parts[2]), {})[parts[3]] = raw
    bundles = {}
    for (_, name), files in sorted(found.items()):
        if set(files) == set(BUNDLE_FILES):
            bundles[name] = files
    return bundles


def melee_skill_maximum(formula):
    melee = formula.get('melee', {})
    attack, skill = melee.get('attack'), melee.get('skill')
    if type(attack) is not int or type(skill) is not int or not (0 < attack <= 0x7FFFFFFF and 0 < skill <= 0x7FFFFFFF):
        return None
    raw = skill * (attack * 0.05) + attack * 0.5
    if not math.isfinite(raw) or math.ceil(raw) > 0x7FFFFFFF:
        return None
    return math.ceil(raw)


def resolve_effect(effect, formulas, schedule):
    """Return (authored effect, None) or (None, reason)."""
    operation = effect['operation']
    if operation not in ('damage', 'heal'):
        # Non-magnitude donor effects are carried verbatim (no invented semantics).
        verbatim = {k: v for k, v in effect.items() if k not in ('identity', 'presentation')}
        return {'donor_effect_key': effect['identity']['key'], **verbatim}, None
    formula = formulas.get(ref_key(effect['formula']))
    if formula is None:
        return None, f'{operation}_formula_absent'
    kind = formula['kind']
    if kind == 'range':
        minimum, maximum = formula['magnitude']['minimum'], formula['magnitude']['maximum']
        derivation = 'donor_range'
    elif kind == 'caster_magnitude':
        magnitude = schedule.get('magnitude')
        if not magnitude:
            return None, 'caster_magnitude_without_schedule_magnitude'
        minimum, maximum = magnitude['minimum'], magnitude['maximum']
        derivation = 'schedule_magnitude'
    elif kind == 'melee_attack_skill':
        maximum = melee_skill_maximum(formula)
        if maximum is None:
            return None, 'melee_skill_formula_out_of_range'
        minimum = 0
        derivation = 'melee_attack_skill_formula'
    else:
        return None, f'{operation}_formula_kind_unsupported:{kind}'
    if type(minimum) is not int or type(maximum) is not int or not 0 <= minimum <= maximum <= 0xFFFFFFFF:
        return None, f'{operation}_range_invalid'
    result = {
        'donor_effect_key': effect['identity']['key'], 'operation': operation,
        'damage_type': effect['damage_type'], 'minimum': minimum, 'maximum': maximum,
        'derivation': derivation, 'formula_kind': kind,
        'mitigated_by': effect.get('mitigated_by', []),
    }
    if kind == 'melee_attack_skill':
        result['melee_skill_formula'] = {**formula['melee'], 'engine': MELEE_SKILL_FORMULA}
    if 'affects' in effect:
        result['affects'] = effect['affects']
    return result, None


def derive(creature_record, index, bundle, profile_attack):
    """One attacks[] entry -> (authoring or None, reason or None)."""
    monster = json.loads(bundle['monster.json'])
    deps = json.loads(bundle['dependencies.json'])
    manifest = json.loads(bundle['manifest.json'])
    sources = monster['behavior'].get('attacks', [])
    schedule = sources[index]
    if schedule['interval_ms'] != profile_attack['interval_ms'] \
            or schedule.get('chance_percent', 100) * 10_000 != profile_attack['chance_ppm']:
        return None, 'schedule_differs_from_donor'
    if schedule['ability']['key'].rsplit('/', 1)[-1] != profile_attack['ability']['key'].rsplit('.', 1)[-1].rsplit('/', 1)[-1]:
        return None, 'attack_slot_suffix_differs'
    abilities = {ref_key(a['identity']): a for a in deps['abilities']}
    effects = {ref_key(e['identity']): e for e in deps['effects']}
    formulas = {ref_key(f['identity']): f for f in deps['formulas']}
    ability = abilities.get(ref_key(schedule['ability']))
    if ability is None:
        return None, 'donor_ability_absent'
    refs = ability.get('effects', [])
    if not refs:
        return None, 'donor_ability_has_no_effects'
    resolved = []
    for ref in refs:
        effect = effects.get(ref_key(ref))
        if effect is None:
            return None, 'donor_effect_absent'
        value, reason = resolve_effect(effect, formulas, schedule)
        if reason:
            return None, reason
        resolved.append(value)
    mapped = [e for e in manifest['entries']
              if e.get('destination') == f'/monster/behavior/attacks/{index}' and e.get('status') == 'mapped']
    repos = [s for s in manifest['sources'] if 'repository' in s and 'kind' not in s]
    if len(mapped) == 1 and len(repos) > 1:
        # A bundle may name a second repository for other fields; the donor file path decides.
        repos = [s for s in repos if mapped[0]['source_file'].startswith(REPOSITORY_ROOTS.get(s['repository'], '\0'))]
    if len(mapped) != 1 or len(repos) != 1:
        return None, 'source_provenance_ambiguous'
    authoring = {
        'kind': ability['kind'], 'range_tiles': ability['range_tiles'],
        'needs_target': ability['needs_target'], 'needs_direction': ability['needs_direction'],
        'effects': resolved,
    }
    if 'area' in ability:
        authoring['area'] = ability['area']
    evidence = {
        'source_repository': repos[0]['repository'], 'source_revision': repos[0]['revision'],
        'source_path': mapped[0]['source_file'], 'source_line': mapped[0].get('source_line'),
        'source_field': mapped[0].get('source_field'), 'donor_ability': schedule['ability'],
        'bundle_sha256': {name: sha256(bundle[name]) for name in BUNDLE_FILES},
    }
    schedule_facts = {'interval_ms': schedule['interval_ms'], 'chance_ppm': profile_attack['chance_ppm']}
    for field in ('magnitude', 'range_tiles'):
        if field in schedule:
            schedule_facts[field] = schedule[field]
    return {'authoring': authoring, 'evidence': evidence, 'schedule': schedule_facts}, None


def bundle_lookup(bundles):
    by_name = collections.defaultdict(list)
    meta = {}
    for name, files in bundles.items():
        monster = json.loads(files['monster.json'])
        by_name[monster['creature']['display_name'].casefold()].append(name)
        meta[name] = monster
    return by_name, meta


def pick_bundle(record, by_name, meta):
    """Bind by native slug first, then by unique display name; health must agree."""
    target_key = record['profile']['target']['key']
    slug = target_key.rsplit('creature.', 1)[-1] if 'creature.' in target_key else None
    profile = record['profile']['data']['profile']
    display = profile['details']['display_name'].casefold()
    if slug in meta:
        name = slug
    elif len(by_name.get(display, [])) == 1:
        name = by_name[display][0]
    else:
        return None, 'no_unique_donor_bundle'
    if meta[name]['creature']['stats']['max_health'] != profile['health']:
        return None, 'donor_health_conflict'
    if len(meta[name]['behavior'].get('attacks', [])) != len(record['behavior']['data']['profile']['attacks']):
        return None, 'attack_count_differs_from_donor'
    return name, None


def build(profiles: dict, bundles: dict):
    by_name, meta = bundle_lookup(bundles)
    derived = {}
    for record in profiles['records']:
        behavior = record.get('behavior')
        attacks = behavior['data']['profile'].get('attacks', []) if behavior else []
        if not attacks:
            continue
        creature = record['profile']['target']['key']
        name, reason = pick_bundle(record, by_name, meta)
        for index, attack in enumerate(attacks):
            ref = attack['ability']
            if name is None:
                result, why = None, reason
            else:
                result, why = derive(record, index, bundles[name], attack)
            derived.setdefault(ref['key'], {'ability': ref, 'bindings': []})['bindings'].append(
                (creature, name, index, result, why))
    rows = []
    for key in sorted(derived):
        ability, bindings = derived[key]['ability'], derived[key]['bindings']
        creatures = sorted({b[0] for b in bindings})
        row = {'schema': SCHEMA, 'ability': ability, 'creatures': creatures}
        failures = sorted({b[4] for b in bindings if b[4]})
        distinct = {canonical(b[3]['authoring']) for b in bindings if b[3]}
        if failures:
            row.update(status='unavailable', reason=failures[0], reasons=failures)
        elif len(distinct) != 1:
            row.update(status='unavailable', reason='conflicting_creature_bindings', reasons=['conflicting_creature_bindings'])
        else:
            first = next(b[3] for b in bindings if b[3])
            row.update(status='authored', qualification=QUALIFICATION, **first['authoring'],
                       schedules=sorted({canonical(b[3]['schedule']).decode() for b in bindings}),
                       evidence=first['evidence'])
            row['schedules'] = [json.loads(s) for s in row['schedules']]
        rows.append(row)
    return rows


def serialise(rows):
    return b''.join(canonical(row) + b'\n' for row in rows)


def pin_document(profiles_raw: bytes, profiles: dict, archive_raw: bytes, rows, records_raw: bytes):
    reasons = collections.Counter(r['reason'] for r in rows if r['status'] == 'unavailable')
    return {
        'schema': PIN_SCHEMA,
        'profile_artifact': {'path': PROFILES, 'sha256': sha256(profiles_raw),
                             'schema': profiles['schema'], 'record_count': len(profiles['records']),
                             'source_definitions_sha256': profiles.get('source_definitions_sha256')},
        'source_archive': {'path': ARCHIVE, 'sha256': sha256(archive_raw)},
        'records': {'path': RECORDS, 'schema': SCHEMA, 'sha256': sha256(records_raw), 'count': len(rows),
                   'authored': sum(r['status'] == 'authored' for r in rows),
                   'unavailable': sum(r['status'] == 'unavailable' for r in rows),
                   'unavailable_by_reason': dict(sorted(reasons.items()))},
        'qualification': QUALIFICATION,
        'meaning': ('Numbers are donor-attested (Canary/Crystal pinned revisions) and carry no Global '
                    'verification; records without a closed donor derivation stay explicitly unavailable.'),
    }


def generate(root: Path = ROOT):
    profiles_raw = (root / PROFILES).read_bytes()
    archive_raw = (root / ARCHIVE).read_bytes()
    profiles = json.loads(profiles_raw)
    rows = build(profiles, load_bundles(root / ARCHIVE))
    records_raw = serialise(rows)
    pin = pin_document(profiles_raw, profiles, archive_raw, rows, records_raw)
    return records_raw, (json.dumps(pin, indent=2, sort_keys=True) + '\n').encode()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=ROOT)
    parser.add_argument('--check', action='store_true', help='fail if the committed files differ')
    args = parser.parse_args()
    records_raw, pin_raw = generate(args.root)
    targets = [(args.root / RECORDS, records_raw), (args.root / PIN, pin_raw)]
    if args.check:
        stale = [str(p) for p, raw in targets if not p.exists() or p.read_bytes() != raw]
        if stale:
            raise SystemExit(f'stale monster attack authoring: {stale}')
        return
    for path, raw in targets:
        path.write_bytes(raw)
    print(json.dumps(json.loads(pin_raw)['records'], sort_keys=True))


if __name__ == '__main__':
    main()
