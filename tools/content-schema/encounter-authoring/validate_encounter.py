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


def ranges(value, where=''):
    """Every {'min', 'max'} range inside a document, with its JSON path."""
    if isinstance(value, dict):
        if set(value) == {'min', 'max'}:
            yield where, value
        for key, child in value.items():
            yield from ranges(child, f'{where}/{key}')
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield from ranges(child, f'{where}/{index}')


def walk(actions, where):
    """Every action with its path, including the actions inside one_of branches (D31)."""
    for n, action in enumerate(actions):
        at = f'{where}/{n}'
        yield at, action
        for b, branch in enumerate(action.get('branches', [])):
            yield from walk(branch['actions'], f'{at}/branches/{b}/actions')


CREATURE_TRIGGERS = ('creature_died', 'lethal_damage', 'health_crossed', 'creature_spawned', 'ability_cast', 'damage_taken',
                     'heal_received', 'damage_accumulated', 'item_used', 'stepped_on')
# CW2-1: `triggering` in `teleport.who` and the `in_anchor` subject also works in area triggers, which fire per creature.
TRIGGERING_TRIGGERS = CREATURE_TRIGGERS + ('area_entered', 'area_left')

def creature_trigger(trigger):
    return trigger['kind'] in CREATURE_TRIGGERS or (trigger['kind'] == 'timer_elapsed' and 'each' in trigger)

def triggering_trigger(trigger):
    return creature_trigger(trigger) or trigger['kind'] in ('area_entered', 'area_left')
# CW2-3: what `map_item` with `triggering` forbids: the item it removes is the one the `stepped_on` rule fired on.
MAP_ITEM_TRIGGERING_FORBIDS = ('item', 'into', 'anchor', 'at', 'destination', 'revert_after_ms', 'revert_destination', 'effect',
                               'interaction', 'unless_present')


def area_has_tile(box, minus):
    """Exact inclusive rectangle subtraction with bounded coordinate compression."""
    cuts = [c for c in minus if c['floor'] == box['floor']]
    edges = {}
    for axis in ('x', 'y'):
        low, high = box[axis]
        edges[axis] = sorted({low, high + 1} | {
            max(low, min(high + 1, edge)) for c in cuts for edge in (c[axis][0], c[axis][1] + 1)})
    return any(not any(c['x'][0] <= x <= c['x'][1] and c['y'][0] <= y <= c['y'][1] for c in cuts)
               for x in edges['x'][:-1] for y in edges['y'][:-1])


def semantic(e, catalog):
    errors = [f'{where}: range min exceeds max' for where, r in ranges(e) if r['min'] > r['max']]
    roles = [p['role'] for p in e['participants']]
    anchors = {a['key']: a['kind'] for a in e['anchors']}
    counters = [c['name'] for c in e['state']['counters']]
    flags = [f['name'] for f in e['state']['flags']]
    timers = [t['name'] for t in e['state']['timers']]
    repeating = {t['name'] for t in e['state']['timers'] if t['repeat']}
    abilities = [a['key'] for a in e.get('abilities', [])]
    for label, names in (('encounter ability', abilities), ('participant role', roles), ('anchor', [a['key'] for a in e['anchors']]), ('counter', counters),
                         ('flag', flags), ('timer', timers), ('rule', [r['key'] for r in e['rules']])):
        duplicates = sorted({n for n in names if names.count(n) > 1})
        if duplicates:
            errors.append(f'duplicate {label} {duplicates}')
    for anchor in e['anchors']:
        location = anchor.get('location')
        if location is None:
            continue
        if ('boxes' in location) != (anchor['kind'] == 'area'):
            errors.append(f"anchor {anchor['key']!r}: a {anchor['kind']} needs a {'box' if anchor['kind'] == 'area' else 'point'} location")
        for box in location.get('boxes', []) + location.get('minus', []):
            if any(box[axis][0] > box[axis][1] for axis in ('x', 'y')):
                errors.append(f"anchor {anchor['key']!r}: a box starts after it ends")
        boxes, minus = location.get('boxes', []), location.get('minus', [])
        def overlaps(a, b):
            return a['floor'] == b['floor'] and all(
                max(a[axis][0], b[axis][0]) <= min(a[axis][1], b[axis][1]) for axis in ('x', 'y'))
        for cut in minus:
            if not any(overlaps(cut, b) for b in boxes):
                errors.append(f"anchor {anchor['key']!r}: a minus box overlaps no area box")
        # Partition at rectangle edges, rather than enumerating up to 65536**2 tiles.
        # Membership is constant within each resulting whole-tile rectangle.
        if minus and not any(area_has_tile(b, minus) for b in boxes):
            errors.append(f"anchor {anchor['key']!r}: minus removes the whole area")
    spawned = {a['role'] for r in e['rules'] for _, a in walk(r['actions'], '') if a['kind'] == 'spawn' and 'role' in a}
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
        elif 'spawned' in value:
            pass  # checked with the action list: a preceding spawn of exactly one creature
        elif trigger['kind'] not in ('creature_died', 'lethal_damage', 'damage_taken'):
            errors.append(f'{where}: the killer exists only for death, lethal damage and damage triggers')

    def position(value, where, rule):
        # D46: positions measured from the subject need a trigger fired by one creature.
        relative_to_subject = value in ('subject_position', 'closest_free_tile') or (
            isinstance(value, dict) and ('offset_tiles' in value or 'relative' in value))
        if relative_to_subject and not creature_trigger(rule['trigger']):
            errors.append(f'{where}: a position measured from the subject needs a trigger fired by one creature')
        if isinstance(value, dict):
            if 'anchor' in value:
                need('anchor', value['anchor'], anchors, where)
            if 'random_in' in value:
                need_area(value['random_in'], where)
            if 'role_position' in value:
                role = value['role_position']
                need('role', role, known_roles, where)
                if 'otherwise' not in value and not any(c['kind'] == 'creature_present' and c.get('role') == role and c['present']
                                                        for c in rule['conditions']):
                    errors.append(f'{where}: role_position needs a creature_present condition for that role or an otherwise')
                if 'otherwise' in value and rule['trigger']['kind'] not in ('creature_died', 'lethal_damage'):
                    errors.append(f'{where}: otherwise death_position needs a death or lethal damage trigger')

    for rule in e['rules']:
        where = f'rules/{rule["key"]}'
        trigger = rule['trigger']
        kind = trigger['kind']
        if 'role' in trigger:
            need('role', trigger['role'], known_roles, where + '/trigger')
        if kind == 'damage_taken' and 'base_vocation' in trigger and trigger['source'] != 'player':
            errors.append(f'{where}/trigger: base_vocation needs source player')
        if kind == 'health_crossed' and ('percent' in trigger) == ('health' in trigger):
            errors.append(f'{where}/trigger: health_crossed takes exactly one of percent and health')
        if kind == 'timer_elapsed':
            need('timer', trigger['timer'], timers, where + '/trigger')
            if 'each' in trigger:
                need('role', trigger['each'], known_roles, where + '/trigger/each')
        if kind == 'counter_reached':
            need('counter', trigger['counter'], counters, where + '/trigger')
        if kind == 'phase_entered':
            need('phase', trigger['phase'], e['phases'], where + '/trigger')
        if kind == 'stepped_on' and 'corpse_of' in trigger:
            need('role', trigger['corpse_of'], known_roles, where + '/trigger')
        if kind in ('area_entered', 'area_left'):
            need_area(trigger['anchor'], where + '/trigger')
            if (trigger['who'] == 'role') != ('role' in trigger):
                errors.append(f'{where}/trigger: a role is required exactly when who is role')
        picked = False
        for n, condition in enumerate(rule['conditions']):
            at = f'{where}/conditions/{n}'
            ck = condition['kind']
            if ck == 'counter_compare':
                need('counter', condition['counter'], counters, at)
            elif ck == 'flag':
                need('flag', condition['flag'], flags, at)
            elif ck == 'creature_present':
                if 'role' in condition:
                    need('role', condition['role'], known_roles, at)
                if ('anchor' in condition) == ('near' in condition):
                    errors.append(f'{at}: creature_present takes exactly one of anchor and near')
                if 'anchor' in condition:
                    need_area(condition['anchor'], at)
                if 'near' in condition:
                    if 'role' in condition['near']:
                        need('role', condition['near']['role'], known_roles, at)
                    elif not triggering_trigger(trigger):
                        errors.append(f'{at}: near triggering needs a creature trigger')
                if 'where' in condition and not condition.get('players'):
                    errors.append(f'{at}: where needs players true')
                if 'pick' in condition:
                    if not condition.get('players') or not condition['present']:
                        errors.append(f'{at}: pick needs players true and present true')
                    picked = True
                for child in condition.get('where', []):
                    if child['kind'] not in ('killer_progress', 'in_anchor') or child.get('subject') != {'candidate': True}:
                        errors.append(f'{at}: where permits only candidate progress and anchor predicates')
                    elif child['kind'] == 'in_anchor':
                        need_area(child['anchor'], at + '/where')
            elif ck == 'in_anchor':
                if 'candidate' in condition['subject']:
                    errors.append(f'{at}: candidate is valid only inside where')
                elif 'triggering' in condition['subject']:
                    if not triggering_trigger(trigger):
                        errors.append(f'{at}: in_anchor of triggering needs a trigger fired by one creature or an area trigger')
                else:
                    subject(condition['subject'], at, trigger)
                need_area(condition['anchor'], at)
            elif ck == 'killer_progress' and 'subject' in condition:
                errors.append(f'{at}: candidate is valid only inside where')
            elif ck in ('has_master', 'summon_count', 'has_condition'):
                need('role', condition['role'], known_roles, at)
            elif ck == 'health_percent':
                need('role', condition['role'], known_roles, at)
            elif ck in ('killer_is_player', 'killer_progress') and kind not in ('creature_died', 'lethal_damage', 'damage_taken', 'heal_received'):
                errors.append(f'{at}: {ck} needs a death, lethal damage, damage or heal trigger')
            elif ck == 'chance_from_amount' and kind not in ('damage_taken', 'heal_received'):
                errors.append(f'{at}: chance_from_amount needs a damage or heal trigger')
            elif ck == 'attacker_wears' and kind not in ('damage_taken', 'damage_accumulated', 'lethal_damage', 'heal_received'):
                errors.append(f'{at}: attacker_wears needs a damage or heal trigger')
        def spawned_speaker(actions, base):
            for n, action in enumerate(actions):
                if action.get('subject', {}).get('spawned') and not any(
                        prior['kind'] == 'spawn' and prior['count'] == 1 for prior in actions[:n]):
                    errors.append(f'{base}/{n}: a spawned subject needs an earlier spawn of one creature in the same list')
                for b, branch in enumerate(action.get('branches', [])):
                    spawned_speaker(branch['actions'], f'{base}/{n}/branches/{b}/actions')
        spawned_speaker(rule['actions'], where + '/actions')
        for at, action in walk(rule['actions'], where + '/actions'):
            ak = action['kind']
            for field in ('role',):
                if field in action and ak != 'spawn':
                    need('role', action[field], known_roles, at)
            if 'subject' in action:
                subject(action['subject'], at, trigger)
            if 'at' in action:
                position(action['at'], at, rule)
            if action.get('owner') == 'death_master' and kind not in ('creature_died', 'lethal_damage'):
                errors.append(f'{at}: death_master exists only for death and lethal damage triggers')
            if ak == 'prevent_death' and (kind != 'lethal_damage' or trigger['role'] != action['role']):
                errors.append(f'{at}: prevent_death is valid only in a lethal_damage rule for the same role')
            if ak == 'remove' and sum(field in action for field in ('role', 'all_in', 'triggering')) != 1:
                errors.append(f'{at}: remove takes exactly one of role, all_in and triggering')
            if ak == 'remove' and 'triggering' in action and kind not in CREATURE_TRIGGERS:
                errors.append(f'{at}: remove triggering needs a trigger fired by one creature')
            if ak == 'remove' and 'triggering' in action and trigger.get('who') == 'player':
                errors.append(f'{at}: remove triggering never removes a player')
            if ak == 'remove' and 'keep_summons' in action and 'all_in' not in action:
                errors.append(f'{at}: keep_summons applies only to remove all_in')
            if action.get('health') == 'remembered' and (ak != 'spawn' or 'role' not in action):
                errors.append(f'{at}: remembered health needs a spawn into a named role')
            if ak == 'message':
                need_area(action['to']['players_in'], at)
            if ak == 'spawn_per_player':
                need_area(action['players_in'], at)
                if 'counter' in action:
                    need('counter', action['counter'], counters, at)
            if ak == 'remove' and 'all_in' in action:
                need_area(action['all_in'], at)
            if ak == 'teleport':
                if isinstance(action['to'], str):
                    need('anchor', action['to'], anchors, at)
                    if any(field in action for field in ('after_ms', 'picked_cooldown_ms', 'say', 'warning_effect', 'arrival_effect')):
                        errors.append(f'{at}: delayed teleport fields need picked_position')
                elif not picked:
                    errors.append(f'{at}: picked_position needs an earlier player pick')
                if 'players_in' in action['who']:
                    need_area(action['who']['players_in'], at)
                elif 'triggering' in action['who']:
                    if not triggering_trigger(trigger):
                        errors.append(f'{at}: teleport triggering needs a trigger fired by one creature or an area trigger')
                else:
                    need('role', action['who']['role'], known_roles, at)
            if ak == 'map_item' and 'triggering' in action:
                if action['operation'] != 'remove':
                    errors.append(f'{at}: map_item triggering is valid only with operation remove')
                if kind != 'stepped_on':
                    errors.append(f'{at}: map_item triggering needs a stepped_on trigger')
                extra = [field for field in MAP_ITEM_TRIGGERING_FORBIDS if field in action]
                if extra:
                    errors.append(f'{at}: map_item triggering forbids {extra}')
            elif ak == 'map_item':
                if ('anchor' in action) == ('at' in action):
                    errors.append(f'{at}: map_item takes exactly one of anchor and at')
                if 'anchor' in action:
                    need('anchor', action['anchor'], anchors, at)
                if action.get('at') == 'death_position' and kind not in ('creature_died', 'lethal_damage'):
                    errors.append(f'{at}: a death position exists only for death and lethal damage triggers')
                if action.get('at') == 'subject_position' and kind not in CREATURE_TRIGGERS:
                    errors.append(f'{at}: map_item subject_position needs a trigger fired by one creature')
                if 'unless_present' in action and action['operation'] != 'create':
                    errors.append(f'{at}: unless_present needs operation create')
                if (action['operation'] == 'transform') != ('into' in action):
                    errors.append(f'{at}: map_item transform needs into, create/remove forbid it')
                for field in ('destination', 'revert_destination'):
                    if field in action:
                        need('anchor', action[field], anchors, at)
                if 'revert_destination' in action and 'revert_after_ms' not in action:
                    errors.append(f'{at}: revert_destination needs revert_after_ms')
            if ak == 'counter':
                need('counter', action['counter'], counters, at)
            if ak == 'flag':
                need('flag', action['flag'], flags, at)
            if ak == 'timer':
                need('timer', action['timer'], timers, at)
                if (action['operation'] == 'add') != ('ms' in action):
                    errors.append(f'{at}: a timer add needs exactly its ms')
            if ak == 'attribute':
                if (action['operation'] == 'add') != ('value' in action):
                    errors.append(f'{at}: an attribute add needs a value, a reset takes none')
                if isinstance(action.get('value'), dict):
                    need('counter', action['value']['counter'], counters, at)
            if ak == 'set_phase':
                need('phase', action['phase'], e['phases'], at)
            if ak == 'damage_modifier' and (action['until'] == 'timer') != ('timer' in action):
                errors.append(f'{at}: damage_modifier until timer needs exactly its timer')
            if ak == 'damage_modifier' and 'timer' in action:
                need('timer', action['timer'], timers, at)
            if ak == 'damage_modifier' and isinstance(action['multiplier_percent'], dict):
                name = action['multiplier_percent']['timer_remaining']
                need('timer', name, timers, at)
                if name in repeating:
                    errors.append(f'{at}: timer_remaining needs a timer that does not repeat')
            if ak == 'cast':
                if ('ability' in action) == ('encounter_ability' in action):
                    errors.append(f'{at}: cast takes exactly one of ability and encounter_ability')
                if 'encounter_ability' in action:
                    need('encounter ability', action['encounter_ability'], abilities, at)
                if action['at'] == 'death_position' and kind not in ('creature_died', 'lethal_damage'):
                    errors.append(f'{at}: a death position exists only for death and lethal damage triggers')
            if ak == 'emit_outcome':
                need('outcome', action['outcome'], e['outcomes'], at)
                if (action['credited'] == 'players_in_anchor') != ('anchor' in action):
                    errors.append(f'{at}: players_in_anchor credit needs exactly its anchor')
                if 'anchor' in action:
                    need_area(action['anchor'], at)
                if action['credited'] == 'killer' and kind not in ('creature_died', 'lethal_damage', 'damage_taken'):
                    errors.append(f'{at}: killer credit needs a death, lethal damage or damage trigger')
                if action['credited'] == 'party' and kind not in ('creature_died', 'lethal_damage'):
                    errors.append(f'{at}: party credit needs a death or lethal damage trigger')
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
    if not isinstance(manifest.get('covers'), dict):
        errors.append('manifest: covers must map each transcribed Canary event to the creature keys it resolves')
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
