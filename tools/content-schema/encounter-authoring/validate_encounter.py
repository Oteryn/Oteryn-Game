"""Validate an Encounter authoring document: JSON Schema plus the semantic rules of the format (§9).

Usage: python validate_encounter.py ENCOUNTER.json [--manifest MANIFEST.json] [--catalog CATALOG.json]
Prints one JSON report; the exit code is 1 when the document is invalid.
"""
import argparse
import json
import sys
from pathlib import Path

import jsonschema

ROOT = Path(__file__).resolve().parent
MANIFEST_STATUS = ('mapped', 'approved_omission', 'unresolved_semantics')


def refs(value):
    """Every {'family', 'key', 'revision'} reference inside a document."""
    if isinstance(value, dict):
        if set(value) == {'family', 'key', 'revision'}:
            yield value
        for child in value.values():
            yield from refs(child)
    elif isinstance(value, list):
        for child in value:
            yield from refs(child)


def semantic(e, catalog):
    errors = []
    roles = [p['role'] for p in e['participants']]
    anchors = {a['key']: a['kind'] for a in e['anchors']}
    counters = [c['name'] for c in e['state']['counters']]
    flags = [f['name'] for f in e['state']['flags']]
    timers = [t['name'] for t in e['state']['timers']]
    for label, names in (('participant role', roles), ('anchor', [a['key'] for a in e['anchors']]), ('counter', counters),
                         ('flag', flags), ('timer', timers), ('rule', [r['key'] for r in e['rules']])):
        duplicates = sorted({n for n in names if names.count(n) > 1})
        if duplicates:
            errors.append(f'duplicate {label} {duplicates}')
    spawned = {a['role'] for r in e['rules'] for a in r['actions'] if a['kind'] == 'spawn' and 'role' in a}
    known_roles = set(roles) | spawned

    def need(kind, name, pool, where):
        if name not in pool:
            errors.append(f'{where}: unknown {kind} {name!r}')

    def need_area(name, where):
        need('anchor', name, anchors, where)
        if anchors.get(name) == 'point':
            errors.append(f'{where}: anchor {name!r} is a point, an area is required')

    def subject(value, where, trigger):
        if 'role' in value:
            need('role', value['role'], known_roles, where)
        elif trigger['kind'] not in ('creature_died', 'lethal_damage', 'damage_taken'):
            errors.append(f'{where}: the killer exists only for death, lethal damage and damage triggers')

    def position(value, where):
        if isinstance(value, dict):
            if 'anchor' in value:
                need('anchor', value['anchor'], anchors, where)
            if 'random_in' in value:
                need_area(value['random_in'], where)

    for rule in e['rules']:
        where = f'rules/{rule["key"]}'
        trigger = rule['trigger']
        kind = trigger['kind']
        if 'role' in trigger:
            need('role', trigger['role'], known_roles, where + '/trigger')
        if kind == 'timer_elapsed':
            need('timer', trigger['timer'], timers, where + '/trigger')
        if kind == 'counter_reached':
            need('counter', trigger['counter'], counters, where + '/trigger')
        if kind == 'phase_entered':
            need('phase', trigger['phase'], e['phases'], where + '/trigger')
        if kind in ('area_entered', 'area_left'):
            need_area(trigger['anchor'], where + '/trigger')
            if (trigger['who'] == 'role') != ('role' in trigger):
                errors.append(f'{where}/trigger: a role is required exactly when who is role')
        for n, condition in enumerate(rule['conditions']):
            at = f'{where}/conditions/{n}'
            ck = condition['kind']
            if ck == 'counter_compare':
                need('counter', condition['counter'], counters, at)
            elif ck == 'flag':
                need('flag', condition['flag'], flags, at)
            elif ck == 'creature_present':
                need('role', condition['role'], known_roles, at)
                need_area(condition['anchor'], at)
            elif ck == 'in_anchor':
                subject(condition['subject'], at, trigger)
                need_area(condition['anchor'], at)
            elif ck == 'health_percent':
                need('role', condition['role'], known_roles, at)
            elif ck in ('killer_is_player', 'killer_progress') and kind not in ('creature_died', 'lethal_damage', 'damage_taken'):
                errors.append(f'{at}: {ck} needs a death, lethal damage or damage trigger')
            elif ck == 'attacker_wears' and kind not in ('damage_taken', 'damage_accumulated', 'lethal_damage'):
                errors.append(f'{at}: attacker_wears needs a damage trigger')
        for n, action in enumerate(rule['actions']):
            at = f'{where}/actions/{n}'
            ak = action['kind']
            for field in ('role',):
                if field in action and ak != 'spawn':
                    need('role', action[field], known_roles, at)
            if 'subject' in action:
                subject(action['subject'], at, trigger)
            if 'at' in action:
                position(action['at'], at)
            if ak == 'prevent_death' and (kind != 'lethal_damage' or trigger['role'] != action['role']):
                errors.append(f'{at}: prevent_death is valid only in a lethal_damage rule for the same role')
            if ak == 'remove' and ('role' in action) == ('all_in' in action):
                errors.append(f'{at}: remove takes exactly one of role and all_in')
            if ak == 'remove' and 'all_in' in action:
                need_area(action['all_in'], at)
            if ak == 'teleport':
                need('anchor', action['to'], anchors, at)
                if 'players_in' in action['who']:
                    need_area(action['who']['players_in'], at)
                else:
                    need('role', action['who']['role'], known_roles, at)
            if ak == 'map_item':
                need('anchor', action['anchor'], anchors, at)
                if (action['operation'] == 'transform') != ('into' in action):
                    errors.append(f'{at}: map_item transform needs into, create/remove forbid it')
            if ak == 'counter':
                need('counter', action['counter'], counters, at)
            if ak == 'flag':
                need('flag', action['flag'], flags, at)
            if ak == 'timer':
                need('timer', action['timer'], timers, at)
            if ak == 'set_phase':
                need('phase', action['phase'], e['phases'], at)
            if ak == 'damage_modifier' and (action['until'] == 'timer') != ('timer' in action):
                errors.append(f'{at}: damage_modifier until timer needs exactly its timer')
            if ak == 'damage_modifier' and 'timer' in action:
                need('timer', action['timer'], timers, at)
            if ak == 'emit_outcome':
                need('outcome', action['outcome'], e['outcomes'], at)
    if catalog is not None:
        declared = {(r['family'], r['key'], r['revision']) for r in catalog['definitions']}
        for ref in refs(e):
            if (ref['family'], ref['key'], ref['revision']) not in declared:
                errors.append(f'unresolved definition {(ref["family"], ref["key"], ref["revision"])}')
    return errors


def check_manifest(manifest):
    errors = []
    if not manifest.get('sources'):
        errors.append('manifest: no sources')
    for n, entry in enumerate(manifest.get('entries', [])):
        if entry.get('status') not in MANIFEST_STATUS:
            errors.append(f'manifest/entries/{n}: status {entry.get("status")!r}')
        if not isinstance(entry.get('source_index'), int) or not 0 <= entry['source_index'] < len(manifest.get('sources', [])):
            errors.append(f'manifest/entries/{n}: source_index out of range')
        if not entry.get('source_lines') or not entry.get('resolution'):
            errors.append(f'manifest/entries/{n}: source_lines and resolution are required')
        if entry.get('status') == 'mapped' and not str(entry.get('destination', '')).startswith('/encounter/'):
            errors.append(f'manifest/entries/{n}: a mapped entry needs an /encounter/ destination')
    return errors


def validate(encounter, catalog=None, manifest=None):
    schema = json.loads((ROOT / 'encounter.schema.json').read_text(encoding='utf-8'))
    errors = [f'schema{"/" + "/".join(map(str, e.absolute_path)) if e.absolute_path else ""}: {e.message}'
              for e in jsonschema.Draft202012Validator(schema).iter_errors(encounter)]
    if not errors:
        errors += semantic(encounter, catalog)
    if manifest is not None:
        errors += check_manifest(manifest)
    return errors


def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    parser.add_argument('encounter', type=Path)
    parser.add_argument('--catalog', type=Path)
    parser.add_argument('--manifest', type=Path)
    args = parser.parse_args()
    load = lambda p: json.loads(p.read_text(encoding='utf-8')) if p else None  # noqa: E731
    errors = validate(load(args.encounter), load(args.catalog), load(args.manifest))
    manifest = load(args.manifest)
    resolved = manifest is not None and all(e['status'] != 'unresolved_semantics' for e in manifest['entries'])
    print(json.dumps({'valid': not errors, 'manifest_resolved': resolved, 'runtime_qualified': False, 'errors': errors}, indent=2))
    sys.exit(1 if errors else 0)


if __name__ == '__main__':
    main()
