#!/usr/bin/env python3
"""Prepare provenance-bound Item fact packets; never admit Items or runtime semantics."""
from __future__ import annotations

import argparse
import collections
import copy
import hashlib
import json
from pathlib import Path

import creature_admission_stage as stage

SCHEMA = 'OTERYN_CREATURE_ITEM_FACT_PREPARATION/v1'
ROUTES = {
    'identity': 'IDENTITY_PROOF_ONLY',
    'presentation': 'ITEM_PRESENTATION_OR_ASSET_BINDING',
    'classification': 'WORLD_OBJECT_OWNER',
    'physical': 'ITEM_TYPED_FIELDS_NATIVE_WEIGHT_UNIT_UNRESOLVED',
    'collision': 'CELL_OR_PLACEMENT_OWNER',
    'container': 'ITEM_TYPED_FIELDS_D3_LIMIT_RECONCILIATION',
    'temporal': 'ITEM_TYPED_FIELDS_D3_RETIREMENT_RECONCILIATION',
}
SOURCE_FIELDS = {
    'identity': {'key', 'revision'},
    'presentation': {'name', 'article', 'asset_binding'},
    'classification': {'is_corpse'},
    'physical': {'weight_centioz', 'movable', 'pickupable'},
    'collision': {'block_walk', 'block_projectiles', 'block_pathfinding'},
    'container': {'capacity'},
    'temporal': {'duration_ms', 'stop_duration', 'decay_action', 'decay_target'},
}
NATIVE_PATHS = {
    'presentation.name': 'presentation.name',
    'physical.weight_centioz': 'physical.weight',
    'physical.movable': 'physical.movable',
    'physical.pickupable': 'physical.pickupable',
    'container.capacity': 'container.capacity',
    'temporal.duration_ms': 'temporal.duration',
    'temporal.stop_duration': 'temporal.stop_duration',
    'temporal.decay_target': 'temporal.decay_target',
}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + '\n')


def field_state(semantics, path):
    """Preserve false, zero, UNKNOWN and CONFLICT without inventing a group value."""
    value = semantics
    for part in path.split('.'):
        if isinstance(value, dict) and 'state' in value:
            if value['state'] != 'KNOWN':
                return copy.deepcopy(value)
            value = value['value']
        if not isinstance(value, dict) or part not in value:
            return {'state': 'UNKNOWN'}
        value = value[part]
    return copy.deepcopy(value) if isinstance(value, dict) and 'state' in value else {'state': 'UNKNOWN'}


def decay_chain(key, projections, bindings):
    """Walk the full local closure; only ITEM-ADD-1 3a self-loops become removal."""
    chain, seen, diagnostics = [], set(), []
    while True:
        if key in seen:
            diagnostics.append({'code': 'NONTERMINAL_DECAY_CYCLE', 'source_item': key})
            break
        seen.add(key)
        if key not in projections:
            diagnostics.append({'code': 'MISSING_SOURCE_DECAY_TARGET', 'source_item': key})
            break
        item = projections[key]
        temporal = copy.deepcopy(item.get('temporal', {}))
        action = temporal.get('decay_action')
        target_ref = temporal.get('decay_target')
        target = target_ref.get('key') if isinstance(target_ref, dict) else None
        reference_valid = action != 'transform' or (
            isinstance(target_ref, dict) and set(target_ref) == {'family', 'key', 'revision'}
            and target_ref['family'] == 'Item' and isinstance(target, str)
            and (target not in projections
                 or target_ref['revision'] == projections[target]['identity']['revision']))
        if action == 'transform' and not reference_valid:
            diagnostics.append({'code': 'DECAY_TARGET_REFERENCE_MISMATCH', 'source_item': key})
        normalized = 'remove' if action == 'transform' and target == key and reference_valid else action
        binding = bindings.get(key)
        step = {'source_item': key, 'native_binding': binding, 'source_temporal': temporal,
                'prepared_action': normalized}
        if normalized != action:
            step['normalization'] = 'ACCEPTED_ITEM_ADD_1_3a_TERMINAL_SELF_DECAY'
        chain.append(step)
        if binding is None:
            diagnostics.append({'code': 'UNRESOLVED_NATIVE_DECAY_BINDING', 'source_item': key})
        if normalized in ('remove', 'transform'):
            duration = temporal.get('duration_ms')
            if type(duration) is not int or duration <= 0:
                diagnostics.append({'code': 'INVALID_DECAY_DURATION', 'source_item': key})
        if normalized in ('remove', 'none'):
            break
        if normalized != 'transform' or not isinstance(target, str):
            diagnostics.append({'code': 'UNSUPPORTED_DECAY_ACTION_OR_TARGET', 'source_item': key})
            break
        key = target
    return {'steps': chain, 'complete': not diagnostics, 'diagnostics': diagnostics,
            'runtime_execution_qualified': False}


def authoring_fragments(item):
    """Existing-schema fragments only, never a complete or promoted Item definition."""
    fragment, unknowns = {}, ['family_profile', 'delivery_task_eligible', 'trade']
    presentation = item.get('presentation', {})
    if 'name' in presentation:
        fragment['display_name'] = presentation['name']
    physical = item.get('physical', {})
    values = {key: physical[key] for key in ('movable', 'pickupable') if key in physical}
    weight = physical.get('weight_centioz')
    if type(weight) is int and weight >= 0:
        # Item authoring schema explicitly defines centi-ounces; native u32 unit remains UNKNOWN.
        values['weight'] = {'value': f'{weight // 100}.{weight % 100:02d}', 'unit': 'oz'}
    if values:
        fragment['physical'] = values
    capacity = item.get('container', {}).get('capacity')
    if type(capacity) is int and 0 < capacity <= 65535:
        fragment['container'] = {'capacity': capacity}
        unknowns.append('container.content_kind')
    temporal = item.get('temporal', {})
    if 'duration_ms' in temporal:
        fragment['temporal'] = {'duration_ms': temporal['duration_ms']}
        unknowns.append('temporal.consumption_mode')
    return {'fields': fragment, 'unknown_required_or_authority_fields': unknowns,
            'complete_definition': False, 'native_promotion_authorized': False,
            'native_weight_unit_conversion_authorized': False}


def prepare(projections, provenance, bindings, native_records):
    packets = []
    for key, item in sorted(projections.items()):
        binding = bindings.get(key)
        native = native_records.get(binding['key']) if binding else None
        diagnostics = []
        if native is None:
            diagnostics.append({'code': 'NO_NATIVE_ITEM_DEFINITION'})
        capacity = item.get('container', {}).get('capacity')
        if capacity is not None and (type(capacity) is not int or capacity < 1 or capacity > 65535):
            diagnostics.append({'code': 'CAPACITY_OUTSIDE_ITEM_AUTHORING_BOUNDS'})
        if item.get('classification', {}).get('is_corpse') and type(capacity) is int and capacity > 16:
            diagnostics.append({'code': 'SOURCE_CAPACITY_EXCEEDS_D3_16', 'source_capacity': capacity,
                                'accepted_runtime_ceiling': 16, 'source_value_preserved': True})
        fields = []
        for group, values in sorted(item.items()):
            if group not in ROUTES:
                raise ValueError(f'unclassified Item group {group}')
            for field, value in sorted(values.items()):
                if field not in SOURCE_FIELDS[group]:
                    raise ValueError(f'unclassified Item field {group}.{field}')
                path = f'{group}.{field}'
                native_path = NATIVE_PATHS.get(path)
                fields.append({'source_path': path, 'source_value': copy.deepcopy(value),
                               'routing': ROUTES[group], 'native_path': native_path,
                               'native_state': field_state((native or {}).get('semantics', {}), native_path)
                                               if native_path else {'state': 'NOT_APPLICABLE'},
                               'native_promotion_authorized': False})
        chain = decay_chain(key, projections, bindings) if 'temporal' in item else None
        if chain:
            diagnostics.extend(chain['diagnostics'])
        packets.append({'schema': SCHEMA, 'status': 'PREPARED_FACTS_NOT_ADMITTED',
                        'source_item': key, 'canonical_binding': binding,
                        'source_projection_authority': 'MONSTER_CAPABILITY_PROJECTION_NOT_ITEM_AUTHORITY',
                        'identity_evidence': provenance[key], 'source_projection': copy.deepcopy(item),
                        'native_definition_exists': native is not None,
                        'native_materializable': (native or {}).get('materializable'),
                        'native_stack_class': (native or {}).get('stack_class'),
                        'fields': fields, 'authoring_fragments': authoring_fragments(item),
                        'decay_closure': chain, 'diagnostics': diagnostics,
                        'admission_authorized': False, 'runtime_qualified': False})
    return packets


def load_inputs(bundle_root, item_map_path, reference_path):
    item_export = json.loads(item_map_path.read_text())
    if item_export['allocation_digest_sha256'] != stage.ITEM_ALLOCATION_SHA256:
        raise stage.StageError('Item allocation map drifted')
    item_map = {row['source_item_id']: row['native_key'] for row in item_export['records']}
    evidence_files = [item_map_path, reference_path, stage.ITEM_ALIASES, stage.ITEM_BINDINGS]
    for path in stage.ITEM_REKEYS:
        evidence_files.append(path)
        facts = json.loads(path.read_text())['source_identity']
        source_id = facts['source_item_id']
        if item_map.get(source_id) != facts['current_native_key']:
            raise stage.StageError('protected Item rekey drifted')
        item_map[source_id] = facts['target_native_key']
    appearance_index, manifests = stage.load_admitted()
    current_ids = {row[0] for row in manifests[appearance_index['newest']]['entries']}
    aliases = json.loads(stage.ITEM_ALIASES.read_text())['entries']
    records = json.loads(reference_path.read_text())['records']
    identity_records = json.loads(stage.REFERENCE.read_text())['records']
    evidence_files.extend([stage.REFERENCE, Path(stage.__file__),
        stage.ROOT / 'tools/content-schema/monster-authoring/canary_batch.py',
        stage.ROOT / 'tools/content-schema/item-authoring/item.schema.json',
        stage.ROOT / 'imports/official/appearance-membership/admitted.json'])
    item_map = stage.resolve_admitted_item_map(item_map, identity_records, aliases,
                                              json.loads(stage.ITEM_BINDINGS.read_text())['bindings'], current_ids)
    alias_map = {row['key']: row['target'] for row in aliases if row['state'] == 'ALIAS'}
    projections, provenance, bindings, bundle_hashes = {}, collections.defaultdict(list), {}, {}
    for path in sorted(bundle_root.glob('*/dependencies.json')):
        manifest_path = path.parent / 'manifest.json'
        manifest = json.loads(manifest_path.read_text())
        bundle_hashes[path.parent.name] = {'dependencies_sha256': digest(path),
                                         'manifest_sha256': digest(manifest_path)}
        for item in json.loads(path.read_text())['items']:
            key = item['identity']['key']
            if key in projections and projections[key] != item:
                raise ValueError(f'conflicting source projections {key}')
            projections[key] = item
            provenance[key].append({'bundle': path.parent.name, 'sources': manifest['sources'],
                                    'input_hashes': bundle_hashes[path.parent.name],
                                    'bundle_projection_entries': [entry for entry in manifest['entries']
                                        if entry.get('source_field') == 'corpse'
                                        or entry.get('source_field', '').startswith('loot')]})
            # The converter's provisional source id is resolved by the existing proof-checked map.
            prefix, source_id_text = key.rsplit('/', 1)
            if prefix != 'canary:item' or not source_id_text.isdecimal():
                raise ValueError(f'unsupported source Item identity {key}')
            source_id = int(source_id_text)
            protected_key = item_map.get(source_id)
            if protected_key:
                native_key = alias_map.get(protected_key, protected_key)
                bindings[key] = {'family': 'Item', 'key': native_key, 'revision': stage.REVISION,
                                 'proof_resolved_key': protected_key}
    evidence = {'files': {str(path): digest(path) for path in evidence_files},
                'appearance_index': appearance_index, 'bundle_hashes': bundle_hashes,
                'allocation_digest_sha256': item_export['allocation_digest_sha256']}
    native = {r['identity']['key']: r for r in records if r['identity']['family'] == 'Item'}
    if len(native) != sum(r['identity']['family'] == 'Item' for r in records):
        raise ValueError('duplicate native Item records')
    for binding in bindings.values():
        record = native.get(binding['key'])
        if record is not None and record['identity']['revision'] != binding['revision']:
            raise ValueError('native Item revision differs from proof binding')
    return projections, provenance, bindings, native, evidence


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bundles', type=Path, required=True)
    parser.add_argument('--item-map', type=Path, required=True)
    parser.add_argument('--native-reference', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    projections, provenance, bindings, native, evidence = load_inputs(
        args.bundles, args.item_map, args.native_reference)
    packets = prepare(projections, provenance, bindings, native)
    hashes, diagnostics = {}, collections.Counter()
    for packet in packets:
        path = args.out / 'items' / (packet['source_item'].replace(':', '_').replace('/', '_') + '.json')
        write(path, packet)
        hashes[str(path.relative_to(args.out))] = digest(path)
        diagnostics.update(row['code'] for row in packet['diagnostics'])
    summary = {'schema': SCHEMA, 'status': 'PREPARED_FACTS_NOT_ADMITTED',
               'items': len(packets), 'corpse_items': sum(p['source_projection']['classification']['is_corpse'] for p in packets),
               'source_fields_preserved': sum(len(p['fields']) for p in packets),
               'source_occurrences': sum(len(p['identity_evidence']) for p in packets),
               'canonical_bindings': sum(p['canonical_binding'] is not None for p in packets),
               'native_definitions': sum(p['native_definition_exists'] for p in packets),
               'complete_decay_closures': sum(p['decay_closure'] is not None and p['decay_closure']['complete'] for p in packets),
               'diagnostics': dict(diagnostics), 'admission_authorized': False,
               'runtime_qualified': False, 'input_evidence': evidence,
               'generator_sha256': digest(Path(__file__)), 'output_sha256': hashes,
               'boundaries': ['A12 identity evidence does not admit Item gameplay semantics.',
                              'ITEM-ADD-1 appearance-only Items retain UNKNOWN native semantics.',
                              'Source capacity is preserved; D3 runtime ceiling remains 16.',
                              'Source decay transforms do not replace accepted D3 terminal retirement.',
                              'Authoring oz conversion is exact; native weight unit mapping remains unresolved.']}
    write(args.out / 'summary.json', summary)
    print(json.dumps({k: v for k, v in summary.items() if k not in ('input_evidence', 'output_sha256')}, indent=2))


if __name__ == '__main__':
    main()
