"""Qualify all weekly delivery items and the bounty talisman in the native provider.

The existing importer reads Crystal's pinned XML and appearance flags. Accepted
canonical semantic facts take precedence; unknown fields use the source importer,
whose unsupported systems stay unknown. The five previously native cohort items
retain every known fact and attribute; qualified source fills only unknown semantics.
No canonical identity, reference admission, client asset or Item owner is created.
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
import build_source_item_profiles as source
from build_spell_item_identities import BINDINGS, binding_index, bind_profiles

PATH = 'tools/content-schema/native-gameplay/task-board-item-profiles.json'
CATALOGUE = 'crates/gameplay/quests/src/quest/task_board_catalogue.rs'
PIN = '201189f871d23597b82caf88fd611454a2ddefb8'
XML = 'data/items/items.xml'
APPEARANCES = 'data/items/appearances.dat'
DELIVERY = 'data/scripts/lib/task_board_delivery_items.lua'
DEFAULTS = 'src/items/items.hpp'
SOURCE_HASHES = {
    XML: '8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509',
    APPEARANCES: '17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1',
}
SCHEMA = 'OTERYN_NATIVE_TASK_BOARD_ITEM_PROFILES/v1'
TALISMAN = 51978
# Exact overlap with the accepted 045b51c2 native provider, not a global fill policy.
EXISTING_NATIVE_IDS = frozenset((3364, 3371, 3557, 10310, 37109))
DELIVERY_CORRECTIONS = [{
    'source_item_id': 5800, 'item_id': 35588, 'amount': 50,
    'delivery_source': DELIVERY + ':64',
    'item_source': XML + ':66376-66379',
    'appearance_source': APPEARANCES,
    'reason': '5800 is immovable and not pickupable; 35588 is the portable grappling hook.',
    'approval': 'https://github.com/Oteryn/Oteryn-Game/pull/2764#issuecomment-6544430997',
}]
WORM_BINDING = {
    'source_key': 'oteryn:source.crystalserver',
    'source_revision': '00ce02a57ca5a12e48f32a3476e37471167e4c3f',
    'identity_namespace': 'ots/item_server_id', 'external_id': '43848',
    'target': {'family': 'Item', 'key': 'oteryn:item.tibia.i43848', 'revision': 'definition-r1'},
    'disposition': 'EXACT',
}
BINDING_REVISION_QUALIFICATIONS = [{
    'item_id': 43848, 'accepted_source_revision': WORM_BINDING['source_revision'],
    'qualified_source_revision': PIN, 'xml_source': XML,
    'appearance_source': APPEARANCES, 'name': 'worm sponge',
}]


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def canonical_bytes(value):
    return json.dumps(value, sort_keys=True, ensure_ascii=False,
                      separators=(',', ':')).encode()


def delivery_items(root=ROOT):
    text = (root / CATALOGUE).read_text()
    body = text.split('pub const DELIVERY_ITEMS:', 1)[1].split('];', 1)[0]
    rows = [(int(item.replace('_', '')), int(amount.replace('_', '')))
            for item, amount in re.findall(
                r'item_id:\s*([0-9_]+),\s*amount:\s*([0-9_]+)', body)]
    if len(rows) != 436 or len({item for item, _ in rows}) != 436:
        raise ValueError('TASK_BOARD_DELIVERY_COHORT')
    if any(not 0 < item <= 65535 or not 0 < amount <= 50 for item, amount in rows):
        raise ValueError('TASK_BOARD_DELIVERY_BOUNDS')
    return rows


def cohort_ids(root=ROOT):
    return {item for item, _ in delivery_items(root)} | {TALISMAN}


def qualify_delivery_rows(rows):
    """Apply the single approved source-data correction, never a wider ID rewrite."""
    if rows.count((5800, 50)) != 1 or any(item == 35588 for item, _ in rows):
        raise ValueError('TASK_BOARD_DELIVERY_CORRECTION_SOURCE_DRIFT')
    return [(35588 if item == 5800 else item, amount) for item, amount in rows]


def bind_qualified_cohort(records, bindings, source_ids):
    """Keep the accepted 43848 revision; independently qualify its current source.

    The shared importer admits only its original ff7 batch. The runtime instead
    requires exact equality with the admitted binding table, including 43848's
    existing 00ce revision. No candidate proof or rewritten identity is needed.
    All other records still pass through the shared importer's unchanged gate.
    """
    exceptional = [r for r in records if r['authoring']['item']['key'] in
                   ('candidate:item/43848', WORM_BINDING['target']['key'])]
    if (len(exceptional) != 1 or 43848 not in source_ids
            or bindings.get('43848') != WORM_BINDING):
        raise ValueError('TASK_BOARD_WORM_BINDING_QUALIFICATION')
    ordinary = [r for r in records if r not in exceptional]
    bound = bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': ordinary},
                          bindings, source_ids)['records']
    record = copy.deepcopy(exceptional[0])
    item = record['authoring']['item']
    if item.get('family') != 'Item' or not isinstance(item.get('revision'), str) or not item['revision']:
        raise ValueError('TASK_BOARD_WORM_AUTHORED_REFERENCE')
    definition = {'family': 'Item', 'production_key': WORM_BINDING['target']['key'],
                  'revision_ref': WORM_BINDING['target']['revision']}
    for field, value in (('production_binding', WORM_BINDING),
                         ('production_definition', definition),
                         ('production_binding_qualification', None)):
        if field in record and record[field] != value:
            raise ValueError('TASK_BOARD_WORM_BINDING_CONFLICT')
        record[field] = copy.deepcopy(value)
    bound.append(record)
    bound.sort(key=lambda row: int(row['production_binding']['external_id']))
    return bound


def canonical_definitions(root, ids):
    wanted = {f'oteryn:item.tibia.i{item}' for item in ids}
    result = {}
    index = json.loads((root / 'content/items/index.json').read_bytes())
    for path in index['shards']:
        for row in json.loads((root / path).read_bytes())['records']:
            definition = row['definition']
            key = definition['identity']['key']
            if key in wanted:
                if key in result:
                    raise ValueError('TASK_BOARD_CANONICAL_DUPLICATE')
                result[key] = (path, definition)
    if set(result) != wanted:
        raise ValueError(f'TASK_BOARD_CANONICAL_MISSING:{sorted(wanted-set(result))}')
    return result


def retain_known(derived, canonical):
    """Fill canonical Unknown fields from source without replacing accepted facts.

    Equipment patterns join by their authored pattern_id; the canonical list of
    patterns and every known constraint remain authoritative.
    """
    if isinstance(canonical, dict):
        if canonical.get('state') == 'UNKNOWN':
            return copy.deepcopy(derived)
        if canonical.get('state') == 'NOT_APPLICABLE':
            return copy.deepcopy(canonical)
        if not isinstance(derived, dict):
            return copy.deepcopy(canonical)
        result = copy.deepcopy(derived)
        for key, value in canonical.items():
            result[key] = retain_known(derived[key], value) if key in derived else copy.deepcopy(value)
        return result
    if isinstance(canonical, list) and isinstance(derived, list):
        if canonical and all(isinstance(v, dict) and 'pattern_id' in v for v in canonical):
            by_id = {v['pattern_id']: v for v in derived
                     if isinstance(v, dict) and 'pattern_id' in v}
            return [retain_known(by_id.get(v['pattern_id'], {}), v) for v in canonical]
    return copy.deepcopy(canonical)


def validate_record(record):
    sem = record['semantics']
    physical = sem['physical']
    stack = sem['stack']
    if (physical.get('state') != 'KNOWN'
            or physical['value']['movable'] != source.K(True)
            or physical['value']['pickupable'] != source.K(True)
            or sem['container'] != source.NA or sem['temporal'] != source.NA
            or stack.get('state') != 'KNOWN'):
        raise ValueError('TASK_BOARD_ITEM_NOT_A_PORTABLE_LEAF:'
                         + record['production_binding']['external_id'])
    stackable = stack['value']['stackable']
    if stackable not in (source.K(True), source.K(False)):
        raise ValueError('TASK_BOARD_UNKNOWN_STACK')
    expected = {'materializable': True,
                'stack_class': 'StackCapable' if stackable['value'] else 'NonStackable',
                'legal_destinations': ['CharacterInventory', 'Ground']}
    if record['admission'] != expected:
        raise ValueError('TASK_BOARD_SOURCE_CANONICAL_ADMISSION_CONFLICT')
    if record['production_binding']['external_id'] == str(TALISMAN):
        pattern = {'pattern_id': 1, 'primary_slot': source.K('EXTRA'),
                   'additional_reserved_slots': source.K([]),
                   'mutually_exclusive_groups': source.K([]),
                   'vocations': source.K([]), 'level': source.K(0),
                   'compatibility_rule': copy.deepcopy(source.NA)}
        if sem['equipment'] != source.K({'patterns': source.K([pattern])}):
            raise ValueError('TASK_BOARD_TALISMAN_EQUIPMENT_CONFLICT')


def capture(crystal, root=ROOT):
    sources = {path: subprocess.check_output(
        ['git', '-C', str(crystal), 'show', f'{PIN}:{path}'])
        for path in (XML, APPEARANCES, DELIVERY, DEFAULTS)}
    for path, expected in SOURCE_HASHES.items():
        if digest(sources[path]) != expected:
            raise ValueError('TASK_BOARD_SOURCE_PIN:' + path)
    defaults = sources[DEFAULTS].decode()
    for declaration in ('int32_t weight = 0', 'uint8_t stackSize = 100',
                        'uint16_t maxItems = 8', 'uint32_t decayTime = 0'):
        if declaration not in defaults:
            raise ValueError('TASK_BOARD_SOURCE_DEFAULTS')
    original_delivery = [(int(item), int(amount)) for item, amount in re.findall(
        r'\{\s*itemId\s*=\s*(\d+),\s*count\s*=\s*(\d+)\s*\}',
        sources[DELIVERY].decode())]
    delivery = qualify_delivery_rows(original_delivery)
    if delivery != delivery_items(root):
        raise ValueError('TASK_BOARD_DELIVERY_SOURCE_DRIFT')
    ids = {item for item, _ in delivery} | {TALISMAN}
    xml = source.xml_index(sources[XML])
    objects = source.engine_items.load_appearance_objects(sources[APPEARANCES])
    legacy_flags = objects[5800]['flags']
    portable_flags = objects[35588]['flags']
    if (not legacy_flags.get('flags.unmove', False) or legacy_flags.get('flags.take', False)
            or portable_flags.get('flags.unmove', False)
            or not portable_flags.get('flags.take', False)
            or not portable_flags.get('flags.cumulative', False)):
        raise ValueError('TASK_BOARD_DELIVERY_CORRECTION_APPEARANCE_DRIFT')
    if (xml[43848].get('name') != 'worm sponge' or objects[43848].get('name') != 'worm sponge'
            or objects[43848]['flags'].get('market.trade_as_object_id') != 43848):
        raise ValueError('TASK_BOARD_WORM_SOURCE_IDENTITY')
    physical = {r['server_item_id']: r for r in source.policies(
        sources[APPEARANCES], sources[XML], ids)}
    bindings = binding_index((root / BINDINGS.relative_to(ROOT)).read_bytes())
    canonical = canonical_definitions(root, ids)
    records = []
    inputs = []
    for item in sorted(ids):
        key = f'oteryn:item.tibia.i{item}'
        path, definition = canonical[key]
        if bindings[str(item)]['target'] != definition['identity']:
            raise ValueError('TASK_BOARD_CANONICAL_BINDING_CONFLICT')
        record = source.profile(item, xml[item], objects[item], physical[item], bindings, None)
        record['semantics'] = retain_known(record['semantics'], definition['semantics'])
        records.append(record)
        inputs.append({'key': key, 'path': path,
                       'definition_sha256': digest(canonical_bytes(definition))})
    bound = bind_qualified_cohort(records, bindings, ids)
    for record in bound:
        validate_record(record)
    return {'schema': SCHEMA, 'source_revision': PIN,
            'sources': [{'path': path, 'sha256': digest(raw)} for path, raw in sources.items()],
            'delivery_corrections': copy.deepcopy(DELIVERY_CORRECTIONS),
            'binding_revision_qualifications': copy.deepcopy(BINDING_REVISION_QUALIFICATIONS),
            'delivery_items': [{'item_id': item, 'amount': amount} for item, amount in delivery],
            'canonical_inputs': inputs, 'records': bound}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    records = overlay['records']
    ids = cohort_ids(root)
    keys = {f'oteryn:item.tibia.i{item}' for item in ids}
    if (overlay['schema'] != SCHEMA or overlay['source_revision'] != PIN
            or overlay.get('delivery_corrections') != DELIVERY_CORRECTIONS
            or overlay.get('binding_revision_qualifications') != BINDING_REVISION_QUALIFICATIONS
            or len(records) != len(keys)
            or {r['production_definition']['production_key'] for r in records} != keys
            or overlay['delivery_items'] != [{'item_id': item, 'amount': amount}
                                             for item, amount in delivery_items(root)]):
        raise ValueError('TASK_BOARD_PROFILE_SET')
    pinned = {r['path']: r['sha256'] for r in overlay['sources']}
    if any(pinned.get(path) != value for path, value in SOURCE_HASHES.items()):
        raise ValueError('TASK_BOARD_SOURCE_PIN')
    definitions = canonical_definitions(root, ids)
    expected = [{'key': key, 'path': definitions[key][0],
                 'definition_sha256': digest(canonical_bytes(definitions[key][1]))}
                for key in sorted(keys, key=lambda key: int(key.rsplit('i', 1)[1]))]
    if overlay['canonical_inputs'] != expected:
        raise ValueError('TASK_BOARD_CANONICAL_DRIFT')
    bindings = binding_index((root / BINDINGS.relative_to(ROOT)).read_bytes())
    bound = bind_qualified_cohort(records, bindings, ids)
    for record in bound:
        validate_record(record)
    document = json.loads(generated[profile_path])
    existing = {r['production_definition']['production_key']: r for r in document['records']}
    for record in bound:
        key = record['production_definition']['production_key']
        if key in existing:
            if (existing[key]['production_binding'] != record['production_binding']
                    or existing[key]['production_definition'] != record['production_definition']):
                raise ValueError('TASK_BOARD_PROFILE_IDENTITY_CONFLICT')
            if int(record['production_binding']['external_id']) in EXISTING_NATIVE_IDS:
                # Existing 10310 has unknown container/temporal facts; the
                # captured source qualifies a leaf. Sliver also needs marketability.
                # Preserve Known/NotApplicable, all attributes and every other row.
                existing[key]['semantics'] = retain_known(
                    record['semantics'], existing[key]['semantics'])
                validate_record(existing[key])
        else:
            document['records'].append(record)
    document['records'].sort(key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = digest(raw)
    generated[manifest_path] = canonical(manifest)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    raw = (json.dumps(capture(args.crystal), indent=2, ensure_ascii=False) + '\n').encode()
    path = ROOT / PATH
    if args.check:
        if path.read_bytes() != raw:
            raise SystemExit('TASK_BOARD_PROFILE_DRIFT')
    else:
        path.write_bytes(raw)
