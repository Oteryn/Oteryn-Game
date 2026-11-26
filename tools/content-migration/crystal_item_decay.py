"""Capture Crystal's ItemType decay and replay it through the native Item provider."""
from __future__ import annotations

import argparse
from collections import Counter, defaultdict
import copy
import hashlib
import json
from pathlib import Path
import subprocess

from authored_static_item_constructors import definitions
from grizzly_shop_profiles import REVISION, SOURCE_HASHES, source
from quest_reward_item_semantics import load_admissions

ROOT = Path(__file__).resolve().parents[2]
BASE = 'f5fcd6c3c5bb4751a50a9af655feadc0ff062143'
CAPTURE = 'tools/content-migration/samples/crystal-item-decay-source.json'
REPORT = 'tools/content-migration/samples/crystal-item-decay-inventory.json'
MANIFEST = 'content/spells.manifest.json'
PROFILES = 'content/items/definitions/spell-native-profiles.json'
UPDATES = 'content/items/definitions/srx1-item-updates.json'
FIELDS = 'tools/content-schema/spell-authoring/samples/native-field-profiles.json'
K, NA, U = source.K, source.NA, source.U
ABSOLUTE = 'DURABLE_ABSOLUTE_DEADLINE'
ACTIVE = 'AUTHORITATIVE_ACTIVE_TIME_BUDGET'
BLOCKERS = ('blocks_movement', 'blocks_projectile', 'immovable_block_solid')
BOUND = 32


def value(fact, default=None):
    return fact.get('value', default) if fact.get('state') == 'KNOWN' else default


def kind(name, attributes, flags, worn):
    if (flags.get('flags.corpse') or flags.get('flags.player_corpse')
            or 'corpsetype' in attributes or name.startswith(('dead ', 'slain '))
            or name in ('unknown corpse', 'remains of a ghost')):
        return 'corpses'
    if any(word in name for word in ('magic wall', 'wild growth', 'energy wall', 'magic barrier')):
        return 'magic walls and similar'
    if attributes.get('type') == 'magicfield' or 'field' in attributes:
        return 'fields'
    if worn:
        return 'rings and amulets with duration' if worn in ('ring', 'necklace') else 'other worn items'
    if (attributes.get('primarytype') == 'light sources'
            or any(word in name for word in ('torch', 'candlestick', 'candle', 'lit lamp', 'lit skull lamp'))):
        return 'light sources'
    if attributes.get('primarytype') == 'food' or 'ice cream cone' in name:
        return 'food'
    return 'other'


def capture(xml, appearances):
    for name, data in [('items.xml', xml), ('appearances.dat', appearances)]:
        if hashlib.sha256(data).hexdigest() != SOURCE_HASHES[name]:
            raise ValueError(f'unexpected Crystal {name} digest')
    nodes = source.xml_index(xml)
    objects = source.engine_items.load_appearance_objects(appearances)
    attrs = {identifier: source.attrs(node) for identifier, node in nodes.items()}
    timed = {i for i, a in attrs.items() if 'duration' in a or 'decayto' in a}
    equip_pairs = {int(a['transformequipto']): (base_id, a.get('slottype') or next(
        (child.get('value') for child in nodes[base_id].findall('./attribute/attribute')
         if child.get('key', '').lower() == 'slot'), None))
        for base_id, a in attrs.items() if 'transformequipto' in a}
    relevant = timed | {base for active, (base, slot) in equip_pairs.items() if active in timed and slot} | {
        int(attrs[i]['decayto']) for i in timed if 'decayto' in attrs[i]
                        and int(attrs[i]['decayto']) > 0}
    physical = {
        r['server_item_id']: r for r in source.policies(appearances, xml, relevant & objects.keys())
    }
    rows = {}
    for identifier in sorted(relevant):
        a = attrs.get(identifier, {})
        obj = objects.get(identifier, {})
        flags = obj.get('flags', {})
        name = nodes[identifier].get('name', '') if identifier in nodes else obj.get('name', '')
        base, slot = equip_pairs.get(identifier, (None, None))
        worn = slot or a.get('slottype') if base or a.get('stopduration') == '1' else None
        row = {'id': identifier, 'name': name, 'kind': kind(name.lower(), a, flags, worn),
               'decayTo': a.get('decayto'), 'duration': a.get('duration'),
               'showduration': a.get('showduration'), 'stopduration': a.get('stopduration')}
        if identifier in timed:
            row['inventory'] = True
        if base is not None and worn:
            row['equip_base'] = base
        if 'transformdeequipto' in a:
            row['de_equip'] = int(a['transformdeequipto'])
        if 'weight' in a:
            row['weight'] = int(a['weight'])
        if identifier in physical:
            p = physical[identifier]
            row['physical'] = {key: p[key] for key in
                               ('movable', 'pickupable', 'block_solid', 'block_projectile', 'has_height')}
            row['field'] = p['magic_field']
            row['container'] = p['container'] or 'containersize' in a
            row['stack'] = bool(flags.get('flags.cumulative'))
            row['tile'] = p['ground']
            row['top_order'] = next((order for flag, order in
                                     [('clip', 1), ('top', 3), ('bottom', 2)]
                                     if flags.get('flags.' + flag)), 0)
        rows[identifier] = row
    # A later corpse stage can lose its corpse appearance flag. Its predecessor
    # still proves the phase belongs to the corpse chain.
    pending = [r for r in rows.values() if r['kind'] == 'corpses']
    while pending:
        target = rows.get(int(pending.pop()['decayTo'] or -1))
        if target and target['kind'] == 'other':
            target['kind'] = 'corpses'
            pending.append(target)
    return {'revision': REVISION, 'sha256': SOURCE_HASHES, 'records': list(rows.values())}


def index_profiles(document):
    return {r['production_definition']['production_key']: r for r in document['records']}


def temporal(row):
    return value(row.get('semantics', {}).get('temporal', U), {})


def duration(row):
    return int((row['duration'] or '0').split(';')[0]) * 1000


def name_key(identifier):
    return f'oteryn:item.tibia.i{identifier}'


def item_ref(canonical, identifier):
    return {k: v for k, v in canonical[name_key(identifier)]['identity'].items()
            if k in ('key', 'revision')}


def read(path):
    return json.loads(path.read_bytes())


def render(document):
    header = json.dumps({k: v for k, v in document.items() if k != 'records'},
                        ensure_ascii=False, indent=2)
    rows = ',\n'.join('    ' + json.dumps(row, ensure_ascii=False, separators=(',', ':'))
                       for row in document['records'])
    return (header[:-2] + ',\n  "records": [\n' + rows + '\n  ]\n}\n').encode()


def status(profile, item, profiles):
    if not profile:
        return False
    time = temporal(profile)
    mode = value(time.get('consumption_mode', U))
    if value(time.get('duration', U), 0) <= 0:
        return False
    if isinstance(time.get('decay_target'), dict) and time['decay_target'].get('state') == 'UNKNOWN':
        # Worn budgets use the existing expiry owner, which treats an omitted
        # target as retirement. Absolute deadlines require an explicit target.
        if mode != ACTIVE:
            return False
    if item['kind'] in ('rings and amulets with duration', 'other worn items'):
        return (mode == ACTIVE
                and item.get('equip_base') not in (3049, 3052, 3098)
                and item.get('equip_base') is not None
                and name_key(item['equip_base']) in profiles)
    if mode != ABSOLUTE:
        return False
    seen = set()
    for _ in range(BOUND):
        ref = profile['production_definition']
        if ref['production_key'] in seen or not profile['admission']['materializable']:
            return False
        seen.add(ref['production_key'])
        fact = profile.get('semantics', {}).get('temporal', U)
        if fact == NA:
            if len(seen) == 1 or item.get('field'):
                return False
            sem = profile['semantics']
            cls = sem.get('classification', U)
            return (sem.get('container', U).get('state') != 'KNOWN'
                    and (cls == NA or value(cls, {}).get('item_type', U) not in (U, K('MAGIC_FIELD')))
                    and value(value(sem.get('physical', U), {}).get('movable', U)) is not None)
        time = value(fact, {})
        if (value(time.get('consumption_mode', U)) != ABSOLUTE
                or value(time.get('duration', U), 0) <= 0
                or value(time.get('stop_duration', U)) is not False):
            return False
        if item['kind'] != 'corpses':
            sem = profile['semantics']
            classification = sem.get('classification', U)
            item_type = value(classification, {}).get('item_type', U)
            if (classification != NA and item_type == U
                    or value(item_type) == 'MAGIC_FIELD' and not item.get('field')
                    or item.get('field') and value(item_type) != 'MAGIC_FIELD'
                    or value(value(sem.get('physical', U), {}).get('movable', U)) is None
                    or profile['admission']['stack_class'] != 'NonStackable'
                    or sem.get('container', U).get('state') == 'KNOWN'
                    or any(profile['attributes'].get(flag) is None for flag in BLOCKERS)):
                return False
        target = time['decay_target']
        if target == NA:
            return True
        target = value(target, {})
        profile = profiles.get(target.get('key'))
        if not profile or profile['production_definition']['revision_ref'] != target['revision']:
            return False
    return False


def captured(root):
    packet = read(root / CAPTURE)
    if packet['revision'] != REVISION or packet['sha256'] != SOURCE_HASHES:
        raise ValueError('Crystal decay capture source mismatch')
    return {r['id']: r for r in packet['records']}


def project(root, document):
    rows = captured(root)
    canonical = definitions(root)
    bindings = {int(b['external_id']): b
                for b in read(root / 'imports/crystalserver/bindings/items.json')['bindings']
                if b['disposition'] == 'EXACT' and b['external_id'].isdecimal()}
    holds = load_admissions(root)
    before = index_profiles(document)
    field_conditions = {r['source']['server_item_id']: r for r in read(root / FIELDS)['profiles']
                        if r['source']['server'] == 'canary'}
    admitted = {}
    worn = {}

    def can_admit(row, terminal, worn_form=False):
        identifier = row['id']
        key = name_key(identifier)
        d = canonical.get(key)
        b = bindings.get(identifier)
        if not d or not b or b['target'] != d['identity'] or d.get('client_projection') != 'ClientSafe':
            return False
        if any(a.get('holds') for a in holds.get(key, ())):
            return False
        if not row.get('physical') or row['tile'] or row['container'] or (row['stack'] and not terminal):
            return False
        current = before.get(key)
        if current:
            if not current['admission']['materializable']:
                return False
            old = temporal(current)
            # Respect a complete published profile, including verified Global
            # overrides. Existing active-time/charge owners retain their state.
            if value(old.get('consumption_mode', U)) == ACTIVE and not worn_form:
                return False
        # A new magic field needs its exact source condition/collision profile.
        return not row['field'] or row['id'] in field_conditions

    def chain(start):
        result = {}
        current = start
        while current not in result:
            row = rows.get(current)
            key = name_key(current)
            if not row or not can_admit(row, terminal=current != start and not duration(row)):
                return {}
            if current != start and row['field'] != rows[start]['field']:
                return {}
            existing = before.get(key)
            if duration(row) == 0:
                if current == start or row['field'] or row['decayTo'] is not None:
                    return {}
                result[current] = NA
                return result
            if (row['decayTo'] is None or int(row['decayTo']) < 0
                    or row['stopduration'] in ('1', 'true') or len(result) == BOUND):
                return {}
            if existing and status(existing, row, before):
                return result
            target = int(row['decayTo'])
            target_identity = canonical.get(name_key(target), {}).get('identity', {})
            if target > 0 and not target_identity:
                return {}
            result[current] = K({
                'consumption_mode': K(ABSOLUTE),
                'decay_target': K({'key': target_identity['key'], 'revision': target_identity['revision']}) if target else NA,
                'duration': K(duration(row)),
                'stop_duration': K(False),
            })
            if not target:
                return result
            current = target
        return {}

    for i, row in rows.items():
        if (row.get('inventory') and row['kind'] not in
                ('corpses', 'rings and amulets with duration', 'other worn items')):
            admitted.update(chain(i))
        elif row['kind'] in ('rings and amulets with duration', 'other worn items'):
            base_id = row.get('equip_base')
            target = int(row['decayTo'] or -1)
            if (not base_id or row.get('de_equip') != base_id or target < 0
                    or base_id in (3049, 3052, 3098)
                    or status(before.get(name_key(i)), row, before)):
                continue
            base = before.get(name_key(base_id)) or canonical.get(name_key(base_id), {})
            equipment = base.get('semantics', {}).get('equipment', U)
            patterns = value(value(equipment, {}).get('patterns', U), [])
            if (not patterns or not any(
                    value(pattern['primary_slot']) and value(pattern['level']) is not None
                    and value(pattern['vocations']) is not None for pattern in patterns)
                    or any(not can_admit(rows[form], terminal=False, worn_form=True)
                           for form in (i, base_id))
                    or (target and (target not in rows
                                    or not can_admit(rows[target], terminal=True)))):
                continue
            worn[i] = (equipment, 'DEEQUIP', base_id)
            worn[base_id] = (equipment, 'EQUIP', i)
            admitted[i] = K({
                'consumption_mode': K(ACTIVE),
                'decay_target': K(item_ref(canonical, target)) if target else NA,
                'duration': K(duration(row)),
                'stop_duration': K(True),
            })
            # The host takes the budget from the active form on equip.
            admitted[base_id] = NA
            if target and name_key(target) not in before:
                admitted[target] = NA

    for i, temporal_fact in sorted(admitted.items()):
        key = name_key(i)
        record = copy.deepcopy(before.get(key))
        row = rows[i]
        physical = row['physical']
        d = canonical[key]
        if record is None:
            identity = d['identity']
            record = {
                'authoring': {'item': identity},
                'semantics': copy.deepcopy(d.get('semantics', {})),
                'attributes': {'speed_bonus': None, 'field_replaceable': None},
                'admission': {'materializable': True,
                              'stack_class': 'StackCapable' if row['stack'] else 'NonStackable',
                              'legal_destinations': ['CharacterInventory', 'Ground']
                              if physical['movable'] and physical['pickupable'] else ['Ground']},
                'production_definition': {'family': 'Item', 'production_key': key,
                                          'revision_ref': identity['revision']},
                'production_binding': bindings[i],
                'production_binding_qualification': None,
            }
        semantics = record['semantics']
        semantics['temporal'] = temporal_fact
        if i in worn:
            equipment, transform_kind, target = worn[i]
            semantics['equipment'] = equipment
            targets = value(semantics.get('use_transform', U), {}).get('targets', [])
            targets = [edge for edge in targets if edge['kind'] != transform_kind]
            targets.append({'kind': transform_kind, 'target': K(item_ref(canonical, target))})
            semantics['use_transform'] = K({'targets': targets})
        stack = semantics.get('stack', U)
        if not row['stack']:
            semantics['stack'] = K({'stackable': K(False), 'stack_max': NA})
        elif not value(stack, {}).get('stack_max', U).get('value'):
            semantics['stack'] = K({'stackable': K(True), 'stack_max': K(100)})
        semantics['container'] = NA
        old_physical = value(semantics.get('physical', U), {})
        semantics['physical'] = K({
            'movable': K(physical['movable']),
            'pickupable': K(physical['pickupable']),
            'weight': old_physical.get('weight', K(row['weight']) if 'weight' in row else U),
        })
        if row['field']:
            semantics['classification'] = K({'item_type': K('MAGIC_FIELD'), 'capabilities': U})
            record['attributes']['field_condition'] = field_conditions[i]
            # A movable or projectile-blocking field already has an owner's exact
            # profile. The newly admitted ordinary fire is replaceable in Crystal.
            if record['attributes'].get('field_replaceable') is None:
                record['attributes']['field_replaceable'] = True
        elif value(semantics.get('classification', U), {}).get('item_type', U) == U:
            semantics['classification'] = K({'item_type': NA, 'capabilities': U})
        order = row['top_order']
        flags = {
            'blocks_movement': physical['block_solid'],
            'blocks_projectile': physical['block_projectile'],
            'immovable_block_solid': physical['block_solid'] and not physical['movable'],
            'has_height': physical['has_height'],
            'always_on_top': order != 0,
            'always_on_top_order': order,
        }
        for flag, observed in flags.items():
            if record['attributes'].get(flag) is None:
                record['attributes'][flag] = observed
        before[key] = record
    return list(before.values()), admitted


def gap(row, rows):
    if row['kind'] == 'corpses':
        return 'separate corpse owner'
    if row['decayTo'] is None or int(row['decayTo']) < 0 or not duration(row):
        return 'Crystal has no runnable ItemType deadline'
    if row.get('container'):
        return 'decaying container requires atomic descendant retirement or successor custody'
    if row['kind'] in ('rings and amulets with duration', 'other worn items'):
        return 'worn-duration pair or equipment effect is unqualified'
    if row.get('tile'):
        return 'terrain replacement uses a separate owner'
    seen = set()
    cursor = row
    while cursor and duration(cursor):
        if cursor['id'] in seen:
            return 'cyclic source chain cannot use the finite stage receipt owner'
        seen.add(cursor['id'])
        cursor = rows.get(int(cursor['decayTo'] or -1))
    if row.get('stack'):
        return 'stacked decay lacks split and merge deadline proofs'
    return 'native Item, collision facts, or chain target unqualified'


def inventory(root, before, after, projected):
    canonical = definitions(root)
    rows = captured(root)
    results = []
    counts = defaultdict(lambda: {'before': Counter(), 'after': Counter()})
    for identifier, item in rows.items():
        if not item.get('inventory'):
            continue
        key = name_key(identifier)
        result = {k: item[k] for k in ('id', 'name', 'kind', 'duration',
                                       'decayTo', 'showduration', 'stopduration')}
        for label, profiles in [('before', before), ('after', after)]:
            if status(profiles.get(key), item, profiles):
                state = 'qualified and runnable'
            elif key in canonical or key in profiles:
                state = 'present but unqualified'
            else:
                state = 'absent'
            result[label] = state
            counts[item['kind']][label][state] += 1
        if result['after'] != 'qualified and runnable':
            result['gap'] = gap(item, rows)
        result['projected'] = identifier in projected
        results.append(result)
    return {'revision': REVISION, 'counts': dict(sorted(counts.items())),
            'sources': {
                'eligibility': 'src/items/item.cpp:3377-3392',
                'removal': 'src/items/decay/decay.cpp:184-194',
                'container_custody': ['src/game/game.cpp:2626-2637',
                    'src/items/item.cpp:496-501', 'src/items/containers/container.cpp:87-106',
                    'src/items/containers/container.hpp:310'],
            },
            'records': sorted(results, key=lambda r: (r['kind'], r['id']))}


def apply(root, items, order):
    items['records'], admitted = project(root, items)
    items['records'].sort(key=order)
    return admitted


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--xml', type=Path)
    parser.add_argument('--appearances', type=Path)
    parser.add_argument('--project', action='store_true',
                        help='report expected provider state before PC regeneration')
    args = parser.parse_args()
    if bool(args.xml) != bool(args.appearances):
        parser.error('--xml and --appearances must be supplied together')
    if args.xml:
        packet = capture(args.xml.read_bytes(), args.appearances.read_bytes())
        (ROOT / CAPTURE).write_bytes(render(packet))
    base_profiles, base_updates = (
        index_profiles(json.loads(subprocess.check_output(
            ['git', '-C', str(ROOT), 'show', f'{BASE}:{path}'])))
        for path in (PROFILES, UPDATES)
    )
    before = base_profiles | base_updates
    if args.project:
        records, _ = project(ROOT, {'records': list(base_profiles.values())})
        after = index_profiles({'records': records}) | base_updates
    else:
        after = index_profiles(read(ROOT / PROFILES)) | index_profiles(read(ROOT / UPDATES))
    projected = {r['id'] for r in captured(ROOT).values()
                 if before.get(name_key(r['id'])) != after.get(name_key(r['id']))}
    report = inventory(ROOT, before, after, projected)
    report['after_basis'] = 'projected' if args.project else 'published profiles'
    (ROOT / REPORT).write_bytes(render(report))


if __name__ == '__main__':
    main()
