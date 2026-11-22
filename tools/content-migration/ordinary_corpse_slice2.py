"""Apply the selected Crystal 201189f8 corpse custody, bindings and required loot."""

import hashlib
import json

from boss_corpse_profiles import PIN, SOURCE_HASHES

CAPTURE = 'tools/content-schema/native-gameplay/ordinary-corpse-slice2.json'


def capture(root):
    source = json.loads((root / CAPTURE).read_bytes())
    if source['source_revision'] != PIN or source['sources'] != SOURCE_HASHES:
        raise ValueError('ordinary corpse source revision or inputs changed')
    return source


def extend_references(records, root):
    source = capture(root)
    existing = {record['identity']['key']: record for record in records}
    directory = root / 'imports/official/appearance-membership'
    admitted = json.loads((directory / 'admitted.json').read_bytes())
    client = next(row for row in admitted['files'] if row['label'] == 'client-15.30')
    raw = (directory / client['manifest']).read_bytes()
    if hashlib.sha256(raw).hexdigest() != client['manifest_sha256']:
        raise ValueError('ordinary corpse client appearance membership changed')
    appearances = {row[0] for row in json.loads(raw)['entries']}
    for binding in source['creature_bindings'].values():
        key = f"oteryn:item.tibia.i{binding['item']}"
        if binding['item'] not in appearances:
            raise ValueError(f'ordinary corpse has no client appearance: {key}')
        definition = {
            'identity': {'family': 'Item', 'key': key, 'revision': 'definition-r1'},
            'kind': 'Item',
            'client_projection': 'ClientSafe',
            'materializable': False,
            'stack_class': 'Unknown',
        }
        if key not in existing:
            records.append(definition)
            existing[key] = definition
        elif existing[key] != definition:
            raise ValueError(f'ordinary corpse already has a reference owner: {key}')


def bind_creatures(root, creatures):
    source = capture(root)
    profiles = {row['profile']['target']['key']: row['profile']['data']['profile']
                for row in creatures['records']}
    for creature, observation in source['creature_bindings'].items():
        details = profiles[creature]['details']
        if details.get('corpse_item') is not None:
            raise ValueError(f'ordinary corpse already has a creature binding: {creature}')
        details['corpse_item'] = {
            'family': 'Item',
            'key': f"oteryn:item.tibia.i{observation['item']}",
            'revision': 'definition-r1',
        }
    for creature, item in source['admissions'].items():
        if profiles[creature]['details']['corpse_item'] != {
            'family': 'Item', 'key': f'oteryn:item.tibia.i{item}', 'revision': 'definition-r1',
        }:
            raise ValueError(f'ordinary corpse creature binding changed: {creature}')


def admit_items(root, items, order):
    source = capture(root)
    selected = set(source['admissions'].values())
    selected.update(binding['item'] for binding in source['creature_bindings'].values())
    selected.update(source['required_loot'])
    records = source['records']
    if [int(record['production_binding']['external_id']) for record in records] != sorted(selected):
        raise ValueError('ordinary corpse Item capture differs from the selected creatures')

    existing = {row['production_definition']['production_key'] for row in items['records']}
    for record in records:
        key = record['production_definition']['production_key']
        if key in existing:
            raise ValueError(f'ordinary corpse already has native Item custody: {key}')
        existing.add(key)
        items['records'].append(record)
    items['records'].sort(key=order)
