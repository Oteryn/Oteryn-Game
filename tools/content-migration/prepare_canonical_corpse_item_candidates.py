#!/usr/bin/env python3
"""Prepare exact-epoch canonical Item candidates from Item XML, without admission."""
from __future__ import annotations
import argparse
import collections
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import xml.etree.ElementTree as ET

import prepare_creature_item_fact_packets as facts

EPOCHS = ('ff7ede593c69d4c658b382c97443e8155926924a',
          '00ce02a57ca5a12e48f32a3476e37471167e4c3f')
ITEM_XML = 'data/items/items.xml'
APPEARANCES = 'data/items/appearances.dat'
SCHEMA = 'OTERYN_CANONICAL_CORPSE_ITEM_CANDIDATE_PREPARATION/v1'
UNKNOWN = {'state': 'UNKNOWN'}
GROUP_FIELDS = {'presentation': ('name', 'description'),
                'container': ('capacity',), 'physical': ('weight', 'movable', 'pickupable'),
                'temporal': ('consumption_mode', 'duration', 'stop_duration', 'decay_target')}


def parse_items(data, diagnostics=None):
    """Preserve duplicates and nested XML; never assume absent engine defaults."""
    records = collections.defaultdict(list)
    for element in ET.fromstring(data).findall('item'):
        attr = dict(element.attrib)
        if 'id' in attr:
            identifiers = [int(attr['id'])]
        elif 'fromid' in attr and 'toid' in attr:
            start, end = int(attr['fromid']), int(attr['toid'])
            if not 0 <= start <= end <= 2**32-1:
                if diagnostics is not None:
                    diagnostics.append({'code': 'UNEXPANDED_INVALID_SOURCE_XML_RANGE', 'attributes': attr})
                continue
            identifiers = range(start, end + 1)
        else:
            raise ValueError('Item XML identity is absent')
        record = {'attributes': attr, 'children_xml': [ET.tostring(e, encoding='unicode') for e in element],
                  'fields': collections.defaultdict(list)}
        for child in element.findall('attribute'):
            record['fields'][child.get('key', '').lower()].append(dict(child.attrib))
        record['fields'] = dict(record['fields'])
        for identifier in identifiers:
            records[identifier].append(record)
    return dict(records)


def parse_appearances(data, fields):
    records = {}
    for number, raw in fields(data):
        if number != 1:
            continue
        value = {'flags': {'unmove': False, 'take': False, 'corpse': False, 'player_corpse': False}}
        for field, payload in fields(raw):
            if field == 1:
                value['id'] = payload
            elif field == 3:
                for flag, bit in fields(payload):
                    if flag in (14, 18, 42, 43):
                        if bit not in (0, 1):
                            raise ValueError('appearance flag is not boolean')
                        value['flags'][{14: 'unmove', 18: 'take', 42: 'corpse', 43: 'player_corpse'}[flag]] = bool(bit)
            elif field == 4:
                value['name'] = payload.decode('utf-8')
        if 'id' not in value or value['id'] in records:
            raise ValueError('duplicate or absent appearance identity')
        records[value['id']] = value
    return records


def scalar(record, key, diagnostics):
    rows = record['fields'].get(key, [])
    if not rows:
        return None
    if len(rows) != 1 or set(rows[0]) != {'key', 'value'}:
        diagnostics.append({'code': 'XML_ATTRIBUTE_NOT_SINGLE_SCALAR', 'field': key})
        return None
    return rows[0]['value']


def integer(raw, field, diagnostics, maximum=2**64-1):
    if raw is None:
        return None
    try:
        value = int(raw)
        if str(value) != raw or not 0 <= value <= maximum:
            raise ValueError()
        return value
    except (ValueError, TypeError):
        diagnostics.append({'code': 'XML_INTEGER_NOT_REPRESENTABLE', 'field': field, 'raw': raw})
        return None


def boolean(raw, field, diagnostics):
    if raw is None:
        return None
    if raw in ('0', '1'):
        return raw == '1'
    diagnostics.append({'code': 'XML_BOOLEAN_NOT_0_OR_1', 'field': field, 'raw': raw})
    return None


def promotion_row(key, source_id, path, value):
    """Exact shape and bounds of existing decode_item_semantic_promotion_value routes."""
    if path == 'presentation.name':
        if not isinstance(value, str) or not value or len(value.encode('utf-8')) > 2048:
            raise ValueError('invalid TEXT')
        kind = 'TEXT'
    elif path == 'container.capacity':
        if type(value) is not int or not 0 <= value <= 65535:
            raise ValueError('invalid CAPACITY_U16')
        kind = 'CAPACITY_U16'
    else:
        raise ValueError('unaccepted promotion route')
    return {'field_path': path, 'native_key': key, 'source_item_id': source_id,
            'source_value': value, 'typed_value': {'kind': kind, 'value': value}}


def exact_bindings(rows):
    result = {}
    for row in rows:
        if row['source_key'] != 'oteryn:source.crystalserver' or row['source_revision'] not in EPOCHS:
            continue
        if row['identity_namespace'] != 'ots/item_server_id' or row['disposition'] != 'EXACT':
            continue
        target = row['target']
        if set(target) != {'family', 'key', 'revision'} or target['family'] != 'Item':
            raise ValueError('invalid canonical Item binding')
        identity = row['source_revision'], int(row['external_id'])
        if identity in result:
            raise ValueError('duplicate exact source binding')
        result[identity] = copy.deepcopy(target)
    return result


def prepare_item(source_id, scope_binding, tables, bindings, native_records, appearances=None):
    diagnostics, rows, proposed = [], [], {}
    result = {'schema': SCHEMA, 'source_item_id': source_id, 'canonical_binding': scope_binding,
              'source_authority': 'PINNED_ITEM_XML_NOT_MONSTER_PROJECTION',
              'admission_authorized': False, 'materializable_authorized': False,
              'runtime_qualified': False, 'family_profile': None,
              'canonical_item_contract_resolved': False}
    native = native_records.get((scope_binding or {}).get('key'))
    if native and native['identity']['revision'] != scope_binding['revision']:
        raise ValueError('canonical native revision differs')
    source_candidates = []
    for epoch in EPOCHS:
        target = bindings.get((epoch, source_id))
        # A12 aliases may resolve an old protected key; both keys have proof in the packet.
        accepted_keys = {(scope_binding or {}).get('key'), (scope_binding or {}).get('proof_resolved_key')}
        matched = bool(target and target['family'] == 'Item' and scope_binding and scope_binding['family'] == 'Item' and target['key'] in accepted_keys
                       and target['revision'] == scope_binding['revision'])
        source_candidates.append({'revision': epoch, 'xml_records': tables[epoch].get(source_id, []),
                                  'exact_binding': target, 'canonical_binding_matches': matched,
                                  'appearance': (appearances or {}).get(epoch, {}).get(source_id)})
    result['source_candidates'] = source_candidates
    # Epoch selection follows exact existing bindings, never numeric-key agreement.
    qualified = [r for r in source_candidates if r['canonical_binding_matches'] and len(r['xml_records']) == 1]
    selected = qualified[-1] if qualified else None
    if not selected or native is None:
        diagnostics.append({'code': 'NO_EXACT_BOUND_SINGLE_ITEM_XML_RECORD'})
        result.update(diagnostics=diagnostics, promotion_candidates=[], native_fragment=None)
        return result
    result['selected_source_revision'] = epoch = selected['revision']
    record = selected['xml_records'][0]
    appearance = (appearances or {}).get(epoch, {}).get(source_id)
    result['source_appearance'] = appearance
    name = record['attributes'].get('name', (appearance or {}).get('name'))
    if name is not None:
        try:
            rows.append(promotion_row(scope_binding['key'], source_id, 'presentation.name', name))
            proposed['presentation.name'] = name
        except ValueError:
            diagnostics.append({'code': 'NAME_OUTSIDE_ACCEPTED_DECODER_BOUNDS'})
    capacity = integer(scalar(record, 'containersize', diagnostics), 'containersize', diagnostics, 65535)
    if capacity is not None:
        rows.append(promotion_row(scope_binding['key'], source_id, 'container.capacity', capacity))
        proposed['container.capacity'] = capacity
        if capacity > 16:
            diagnostics.append({'code': 'SOURCE_CAPACITY_EXCEEDS_D3_16', 'source_capacity': capacity,
                                'accepted_runtime_ceiling': 16, 'source_value_preserved': True})
    for field, flag in (('movable', 'unmove'), ('pickupable', 'take')):
        keys = ('pickupable', 'allowpickupable') if field == 'pickupable' else ('movable',)
        explicit = [k for k in keys if k in record['fields']]
        if len(explicit) > 1:
            diagnostics.append({'code': 'AMBIGUOUS_PHYSICAL_XML_ALIASES', 'field': field})
            continue
        if explicit:
            value = boolean(scalar(record, explicit[0], diagnostics), field, diagnostics)
        elif appearance:
            # Pinned proto2 bool getter defaults false; Item XML overrides that getter.
            bit = appearance['flags'][flag]
            value = not bit if field == 'movable' else bit
        else:
            value = None
        if value is not None:
            proposed['physical.' + field] = value
    duration = integer(scalar(record, 'duration', diagnostics), 'duration', diagnostics, 2**32-1)
    if duration is not None:
        proposed['temporal.duration'] = duration * 1000  # XML seconds -> explicit ReferenceMilliseconds.
    stop = boolean(scalar(record, 'stopduration', diagnostics), 'stopduration', diagnostics)
    if stop is not None:
        proposed['temporal.stop_duration'] = stop
    target_id = integer(scalar(record, 'decayto', diagnostics), 'decayto', diagnostics, 65535)
    if target_id is not None:
        if target_id == 0:
            result['source_terminal_action'] = 'remove_if_not_loaded_from_map'
            result['source_terminal_guard'] = {'is_loaded_from_map': 'UNKNOWN_CONTEXT'}
            diagnostics.append({'code': 'NATIVE_TERMINAL_REMOVAL_CARRIER_UNAVAILABLE'})
        elif target_id == source_id:
            result['accepted_item_add_1_3a_preparation_action'] = 'remove'
            diagnostics.append({'code': 'SOURCE_SELF_DECAY_NONTERMINAL_NOT_NATIVE_REMOVAL'})
        else:
            target = bindings.get((epoch, target_id))
            target_record = native_records.get((target or {}).get('key'))
            if target and target_record and target_record['identity']['revision'] == target['revision']:
                proposed['temporal.decay_target'] = {'key': target['key'], 'revision': target['revision']}
            else:
                diagnostics.append({'code': 'EXACT_EPOCH_NATIVE_DECAY_TARGET_UNRESOLVED', 'target_id': target_id})
    fragment = copy.deepcopy((native or {}).get('semantics', {}))
    changes = []
    for path, value in sorted(proposed.items()):
        state = facts.field_state(fragment, path)
        group, leaf = path.split('.')
        if state['state'] == 'UNKNOWN':
            if fragment.get(group, UNKNOWN)['state'] == 'UNKNOWN':
                fragment[group] = {'state': 'KNOWN', 'value': {k: copy.deepcopy(UNKNOWN) for k in GROUP_FIELDS[group]}}
            fragment[group]['value'][leaf] = {'state': 'KNOWN', 'value': value}
            disposition = 'PREPARED_UNKNOWN_LOWERING_CANDIDATE'
            changes.append(path)
        elif state == {'state': 'KNOWN', 'value': value}:
            disposition = 'EXISTING_KNOWN_EQUAL'
        else:
            disposition = 'EXISTING_STATE_PRESERVED_CONFLICT'
            diagnostics.append({'code': disposition, 'field': path, 'existing': state, 'source_value': value})
        for row in rows:
            if row['field_path'] == path:
                row['disposition'] = disposition
                row['decoder_row'] = {k: row[k] for k in ('field_path', 'native_key', 'source_item_id', 'source_value', 'typed_value')}
    diagnostics.append({'code': 'NATIVE_WEIGHT_UNIT_UNQUALIFIED'})
    if duration is not None or target_id is not None:
        diagnostics.append({'code': 'TEMPORAL_MODE_UNQUALIFIED'})
    if appearance and (appearance['flags'].get('corpse') or appearance['flags'].get('player_corpse')):
        diagnostics.append({'code': 'WORLD_OBJECT_CORPSE_ROUTE_REQUIRED_BY_ITEM_SCHEMA_5m'})
    result.update(diagnostics=diagnostics, promotion_candidates=rows,
                  native_fragment={'identity': (native or {}).get('identity'), 'semantics': fragment,
                                   'prepared_fields': changes, 'applied_to_native': False},
                  units={'duration': 'XML_SECONDS_TO_REFERENCE_MILLISECONDS', 'weight': 'UNKNOWN'})
    return result


def canonical_decay_closure(identifier, epoch, tables, bindings):
    steps, seen, diagnostics = [], set(), []
    while identifier not in seen:
        seen.add(identifier)
        records = tables[epoch].get(identifier, [])
        binding = bindings.get((epoch, identifier))
        if len(records) != 1 or binding is None:
            diagnostics.append({'code': 'UNRESOLVED_EXACT_EPOCH_DECAY_STAGE', 'source_item_id': identifier})
            break
        record = records[0]
        target = integer(scalar(record, 'decayto', diagnostics), 'decayto', diagnostics, 65535)
        duration = integer(scalar(record, 'duration', diagnostics), 'duration', diagnostics, 2**32-1)
        action = 'UNKNOWN' if target is None else 'remove_if_not_loaded_from_map' if target == 0 else 'transform'
        steps.append({'source_item_id': identifier, 'binding': binding,
                      'raw_attributes': record['fields'], 'source_action': action,
                      'duration_ms': None if duration is None else duration * 1000})
        if target is None:
            diagnostics.append({'code': 'NO_EXPLICIT_SOURCE_DECAY_ACTION'})
            break
        if not duration:
            diagnostics.append({'code': 'NO_POSITIVE_SOURCE_DECAY_DURATION'})
        if target == 0:
            steps[-1]['terminal_guard'] = {'is_loaded_from_map': 'UNKNOWN_CONTEXT'}
            break
        identifier = target
    else:
        diagnostics.append({'code': 'NONTERMINAL_SOURCE_DECAY_CYCLE', 'source_item_id': identifier})
    return {'steps': steps, 'source_closure_complete': not diagnostics,
            'diagnostics': diagnostics, 'runtime_qualified': False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('repo', 'crystal', 'fact-packets', 'native-reference', 'out'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    sys.path.insert(0, str(args.repo / 'tools/content-schema/monster-authoring'))
    from canary_batch import fields
    source_data, appearance_data, evidence, parsing_findings = {}, {}, {}, {}
    for epoch in EPOCHS:
        data = subprocess.check_output(['git', '-C', str(args.crystal), 'show', f'{epoch}:{ITEM_XML}'])
        parsing_findings[epoch] = []
        source_data[epoch] = parse_items(data, parsing_findings[epoch])
        raw = subprocess.check_output(['git', '-C', str(args.crystal), 'show', f'{epoch}:{APPEARANCES}'])
        appearance_data[epoch] = parse_appearances(raw, fields)
        evidence[epoch] = {'repository': 'zimbadev/crystalserver', 'revision': epoch, 'path': ITEM_XML,
                           'sha256': hashlib.sha256(data).hexdigest(), 'bytes': len(data),
                           'appearances_sha256': hashlib.sha256(raw).hexdigest(), 'appearances_bytes': len(raw), 'engine_rules': {}}
        for relative in ('src/protobuf/appearances.proto', 'src/items/items.cpp', 'src/items/functions/item/item_parse.cpp', 'src/items/decay/decay.cpp', 'src/game/game.cpp'):
            rule = subprocess.check_output(['git', '-C', str(args.crystal), 'show', f'{epoch}:{relative}'])
            evidence[epoch]['engine_rules'][relative] = hashlib.sha256(rule).hexdigest()
    bindings_path = args.repo / 'imports/crystalserver/bindings/items.json'
    bindings = exact_bindings(json.loads(bindings_path.read_text())['bindings'])
    native = {r['identity']['key']: r for r in json.loads(args.native_reference.read_text())['records']
              if r['identity']['family'] == 'Item'}
    scope_summary_path = args.fact_packets / 'summary.json'
    scope = json.loads(scope_summary_path.read_text())
    hashes, counts = {}, collections.Counter()
    for relative, expected in sorted(scope['output_sha256'].items()):
        path = args.fact_packets / relative
        if facts.digest(path) != expected:
            raise ValueError('scope fact packet drifted')
        packet = json.loads(path.read_text())
        identifier = int(packet['source_item'].rsplit('/', 1)[1])
        result = prepare_item(identifier, packet['canonical_binding'], source_data, bindings, native, appearance_data)
        epoch = result.get('selected_source_revision')
        result['canonical_source_decay_closure'] = canonical_decay_closure(identifier, epoch, source_data, bindings) if epoch else None
        result['scope_packet_sha256'] = expected
        result['scope_is_corpse'] = packet['source_projection']['classification']['is_corpse']
        out = args.out / 'items' / f'{identifier}.json'
        facts.write(out, result)
        hashes[str(out.relative_to(args.out))] = facts.digest(out)
        counts['items'] += 1
        counts['xml_bound_items'] += 'selected_source_revision' in result
        closure = result['canonical_source_decay_closure']
        if closure:
            counts['complete_source_decay_closures'] += closure['source_closure_complete']
            counts['source_decay_steps'] += len(closure['steps'])
            counts.update('closure/' + d['code'] for d in closure['diagnostics'])
        counts['decoder_rows'] += len(result['promotion_candidates'])
        counts['prepared_native_fields'] += len((result['native_fragment'] or {}).get('prepared_fields', []))
        counts.update('diagnostic/' + d['code'] for d in result['diagnostics'])
    facts.write(args.out / 'summary.json', {'schema': SCHEMA, 'counts': dict(counts), 'source_evidence': evidence,
        'input_sha256': {str(p): facts.digest(p) for p in (bindings_path, args.native_reference, scope_summary_path)},
        'source_parse_findings': parsing_findings,
        'generator_sha256': facts.digest(Path(__file__)), 'output_sha256': hashes,
        'admission_authorized': False, 'runtime_qualified': False, 'canonical_item_contract_resolved': False})
    print(json.dumps(dict(counts), indent=2))


if __name__ == '__main__':
    main()
