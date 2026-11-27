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
    from boss_conditional_item_reference import definition as qualified_reference

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
        if binding['item'] == 35136:
            definition = qualified_reference(key)
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


def merge_shared_item(existing, record):
    """Preserve both captures when they establish the same source Item."""
    key = record['production_definition']['production_key']
    for field in ('production_definition', 'production_binding', 'production_binding_qualification'):
        if existing.get(field) != record.get(field):
            raise ValueError(f'Conflicting corpse {field}: {key}')
    admission = record['admission']
    # Crystal 4024 is both a shared decay stage and a source-portable body.
    if key == 'oteryn:item.tibia.i4024' and admission['legal_destinations'] == ['Ground']:
        admission = {**admission, 'legal_destinations': ['CharacterInventory', 'Ground']}
    if existing['admission'] != admission:
        raise ValueError(f'Conflicting corpse admission: {key}')
    for field, value in record['semantics'].items():
        if value == {'state': 'UNKNOWN'}:
            continue
        prior = existing['semantics'].get(field)
        if prior is None or prior == {'state': 'UNKNOWN'}:
            existing['semantics'][field] = value
        elif prior != value:
            raise ValueError(f'Conflicting corpse {field}: {key}')
    for field, value in record['attributes'].items():
        if existing['attributes'].setdefault(field, value) != value:
            raise ValueError(f'Conflicting corpse {field}: {key}')


def admit_items(root, items, order, shared_ids, capacity_projections, loot_admissions):
    from ordinary_corpse_profiles import preserve_projected_capacity

    source = capture(root)
    selected = set(source['admissions'].values())
    selected.update(binding['item'] for binding in source['creature_bindings'].values())
    required_loot = set(source['required_loot'])
    selected.update(required_loot)
    records = source['records']
    if [int(record['production_binding']['external_id']) for record in records] != sorted(selected):
        raise ValueError('ordinary corpse Item capture differs from the selected creatures')

    existing = {row['production_definition']['production_key']: row for row in items['records']}
    for record in records:
        key = record['production_definition']['production_key']
        preserve_projected_capacity(record, existing.get(key), capacity_projections)
        if key in existing:
            item = int(record['production_binding']['external_id'])
            if item in required_loot and key in loot_admissions:
                # Preserve the established loot presentation. This capture
                # qualifies the required Item's custody and source identity.
                prior_text = existing[key]['semantics']['presentation']
                observed_text = record['semantics']['presentation']
                prior_name = prior_text.get('value', {}).get('name', {})
                observed_name = observed_text.get('value', {}).get('name', {})
                if prior_name.get('state') != 'KNOWN' or prior_name != observed_name:
                    raise ValueError(f'Conflicting required corpse loot name: {key}')
                record['semantics']['presentation'] = prior_text
            elif item not in shared_ids:
                raise ValueError(f'ordinary corpse already has native Item custody: {key}')
            if item == 35136:
                # The shared Tentacle identity also carries the exact slice-2 authorization row.
                merge_shared_item(record, existing[key])
                existing[key].update(record)
            else:
                merge_shared_item(existing[key], record)
        else:
            items['records'].append(record)
    items['records'].sort(key=order)
