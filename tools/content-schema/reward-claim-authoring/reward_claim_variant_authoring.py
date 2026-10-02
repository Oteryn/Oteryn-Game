"""Canonical variant DATA authoring under D277; native execution stays blocked."""
import copy
import hashlib
import json
from pathlib import Path

import reward_claim_variant_migration as migration

PROFILE = 'authored_variant_v1'
SCHEMA = 'OTERYN_REWARD_CLAIM_VARIANT_SHARD/v1'


def canonical_schema(root):
    """Reuse covered reward vocabulary; this profile cannot admit native execution."""
    vocabulary = migration.read(root, migration.SOURCE_SCHEMA)
    defs = copy.deepcopy(vocabulary['$defs'])
    props = {'identity': {'$ref': '#/$defs/identity'}, 'claim': defs['claim']['properties']['claim'],
        'provenance': {'type': 'object', 'additionalProperties': False,
            'required': ['pilot_key', 'pilot_revision'], 'properties': {
                'pilot_key': {'type': 'string'}, 'pilot_revision': {'type': 'string'}}},
        'definition_profile': {'const': PROFILE},
        'readiness': {'enum': ['waiting_implementation', 'waiting_data']},
        'native_admission': {'const': 'WAITING_IMPLEMENTATION'},
        'quest': {'$ref': '#/$defs/ref'}, 'placements': {'type': 'array', 'minItems': 1,
            'items': {'type': 'object', 'additionalProperties': False,
                'required': ['appearance_tibia_id', 'source_binding', 'reward'], 'properties': {
                    'appearance_tibia_id': {'type': 'integer', 'minimum': 1},
                    'source_binding': {'type': 'object', 'additionalProperties': False,
                        'required': ['project_position', 'legacy_unique_ids'], 'properties': {
                            'project_position': {'$ref': '#/$defs/position'},
                            'legacy_unique_ids': {'type': 'array', 'minItems': 1, 'uniqueItems': True,
                                'items': {'type': 'object', 'additionalProperties': False,
                                    'required': ['server', 'unique_id'], 'properties': {
                                        'server': {'enum': ['canary', 'crystalserver']},
                                        'unique_id': {'type': 'integer', 'minimum': 1}}}}}},
                    'reward': {'$ref': '#/$defs/reward'},
                    'achievement_grant': {'$ref': 'oteryn:schema/reward-claim-source-migration-packet/v1#/properties/records/items/properties/achievement_grants/items/properties/grant_request'}}}},
        'data_holds': {'type': 'array', 'items': {'type': 'object'}},
        'source_variant': {'$ref': 'oteryn:schema/reward-claim-source-migration-packet/v1#/properties/records/items'}}
    return {'$schema': vocabulary['$schema'], '$id': 'oteryn:schema/canonical-reward-claim-variant/v1',
        '$defs': defs, 'type': 'object', 'additionalProperties': False,
        'required': sorted(set(props) - {'quest'}), 'properties': props}



def serialized_item_stacks(reward, items, stack_problem):
    """Serialize authored totals into known admitted stacks; never change random odds.

    Original raw arguments and donor execution remain in source_variant. This
    DATA representation stays blocked at the existing native variant boundary.
    """
    serialized = []
    for quantity in reward.get('items', []):
        item = items[quantity['item']['key']]
        stack = item.get('semantics', {}).get('stack', {})
        maximum = stack.get('value', {}).get('stack_max', {}) if stack.get('state') == 'KNOWN' else {}
        limit = maximum.get('value') if maximum.get('state') == 'KNOWN' else None
        if (item.get('stack_class') == 'StackCapable' and type(limit) is int
                and 1 <= limit <= 100 and stack_problem(item, limit) is None):
            count = quantity['count']
            while count > limit:
                serialized.append({'item': copy.deepcopy(quantity['item']), 'count': limit})
                count -= limit
            serialized.append({'item': copy.deepcopy(quantity['item']), 'count': count})
        else:
            serialized.append(copy.deepcopy(quantity))
    reward['items'] = serialized



def semantic_digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(',', ':')).encode()).hexdigest()


def stack_normalization_proof():
    path = Path(__file__).with_name('reward_stack_normalization.json')
    if hashlib.sha256(path.read_bytes()).hexdigest() != '962a6cde5932fb4e17f457474de721f52749c32a7626ad7cf6d505916949fc69':
        raise ValueError('selected stack normalization proof changed')
    return json.loads(path.read_text())


def stack_normalization_checks():
    proof = stack_normalization_proof()
    return [{'code': 'OTERYN_SELECTED_STACK_SERIALIZATION', 'proof_path':
        'tools/content-schema/reward-claim-authoring/reward_stack_normalization.json',
        'proof_sha256': '962a6cde5932fb4e17f457474de721f52749c32a7626ad7cf6d505916949fc69', 'source_behavior': 'RECORDED_DIVERGENCE',
        'native_admission': 'WAITING_IMPLEMENTATION', 'selected_cases': proof['cases']}]


def derive_definitions(packet, items, stack_problem):
    """No dummy rewards, guessed carrier, or source count promoted to definition charges."""
    out, identities, positions = [], set(), set()
    selected = {c['source_identity']['key']: c for c in stack_normalization_proof()['cases']}
    for row in packet['records']:
        source = row['source_claim']; bindings = {}
        for binding in row['item_bindings']:
            location = (binding['placement_index'], binding['reward_field'], binding['reward_index'])
            if location in bindings:
                raise ValueError('duplicate variant Item binding')
            bindings[location] = binding
        charges = {(c['placement_index'], c['reward_field'], c['reward_index']): c
                   for c in row['charge_dispositions']}
        holds = [{'category': 'source', **copy.deepcopy(g)} for g in row['source_data']['missing']]
        holds += [{'category': 'source', 'code': c['reason'], 'placement_index': c['placement_index'],
                   'reward_field': c['reward_field'], 'reward_index': c['reward_index']}
                  for c in row['charge_dispositions'] if c['data_status'] != 'AUTHORED']
        placements = []
        for pi, placement in enumerate(source['placements']):
            reward = copy.deepcopy(placement['reward'])
            fields = [(field, qi) for field in ('items', 'random_one_of')
                      for qi, _ in enumerate(reward.get(field, []))]
            fields += [(field, None) for field in ('container', 'written_text')
                       if field in reward and (field != 'written_text' or reward[field]['item'] is not None)]
            for field, qi in fields:
                binding = bindings[(pi, field, qi)]; ref = binding['canonical_item']
                if ref is None:
                    raise ValueError('canonical variant Item identity unavailable')
                item = items.get(ref['key'])
                if item is None or item['identity'] != ref:
                    raise ValueError('canonical variant Item identity/revision changed')
                count = 1
                if qi is not None:
                    count = reward[field][qi]['count']; charge = charges.get((pi, field, qi))
                    if charge and charge['normalized_quantity']['state'] == 'KNOWN':
                        count = charge['normalized_quantity']['value']
                    reward[field][qi] = {'item': copy.deepcopy(ref), 'count': count}
                elif field == 'written_text':
                    reward[field]['item'] = copy.deepcopy(ref)
                else:
                    reward[field] = copy.deepcopy(ref)
                # Additive items are checked after canonical stack serialization.
                problem = stack_problem(item, count) if field != 'items' else None
                if problem:
                    holds.append({'category': 'item', 'code': 'ITEM_SEMANTICS_MISSING',
                        'placement_index': pi, 'reward_field': field, 'reward_index': qi,
                        'item': ref['key'], 'reason': problem})
            case = selected.get(source['identity']['key'])
            if case and pi == case['placement_index']:
                if (semantic_digest(source) != case['source_claim_sha256']
                        or semantic_digest(items[case['item_identity']['key']]) != case['item_definition_sha256']):
                    raise ValueError('selected source/Item stack normalization inputs changed')
                serialized_item_stacks(reward, items, stack_problem)
            for qi, quantity in enumerate(reward.get('items', [])):
                problem = stack_problem(items[quantity['item']['key']], quantity['count'])
                if problem:
                    holds.append({'category': 'item', 'code': 'ITEM_SEMANTICS_MISSING',
                        'placement_index': pi, 'reward_field': 'items', 'reward_index': qi,
                        'item': quantity['item']['key'], 'reason': problem})
            witnesses = row['placement_bindings'][pi]['manifest_entries']
            uids = sorted({(w['source'], w['uid']) for entry in witnesses for w in entry['sources']})
            if not uids:
                raise ValueError('canonical variant placement has no exact source UID binding')
            p = {'appearance_tibia_id': int(placement['appearance']['key'].split(':item/', 1)[1]),
                'source_binding': {'project_position': copy.deepcopy(placement['position']),
                    'legacy_unique_ids': [{'server': server, 'unique_id': uid} for server, uid in uids]},
                'reward': reward}
            grants = [g for g in row['achievement_grants'] if g['placement_index'] == pi]
            if grants:
                if len(grants) != 1:
                    raise ValueError('ambiguous canonical Achievement grant')
                p['achievement_grant'] = copy.deepcopy(grants[0]['grant_request'])
            placements.append(p)
        marker = source['identity']['key'].split(':reward-claim/', 1)[1].replace('/', '.')
        definition = {'identity': {'key': 'oteryn:reward-claim.' + marker, 'revision': 'reward-claim-r1'},
            'claim': copy.deepcopy(source['claim']), 'placements': placements,
            'provenance': {'pilot_key': source['identity']['key'], 'pilot_revision': source['identity']['revision']},
            'definition_profile': PROFILE, 'native_admission': 'WAITING_IMPLEMENTATION',
            'readiness': 'waiting_data' if holds else 'waiting_implementation', 'data_holds': holds,
            'source_variant': copy.deepcopy(row)}
        if source.get('quest'):
            definition['quest'] = copy.deepcopy(source['quest'])
        key = definition['identity']['key']
        if key in identities:
            raise ValueError('canonical variant identity collision')
        identities.add(key)
        for p in placements:
            where = tuple(p['source_binding']['project_position'][axis] for axis in ('x', 'y', 'z'))
            if where in positions:
                raise ValueError('canonical variant placement collision')
            positions.add(where)
        out.append({'definition': definition})
    return out


def validator(root):
    import jsonschema
    from referencing import Registry, Resource
    schemas = [migration.read(root, name) for name in (migration.SOURCE_SCHEMA, migration.INTERACTION_SCHEMA)]
    schemas.append(json.loads(Path(__file__).with_name('reward_claim_variant_packet.schema.json').read_text()))
    registry = Registry().with_resources((s['$id'], Resource.from_contents(s)) for s in schemas)
    return jsonschema.Draft202012Validator(canonical_schema(root), registry=registry)


def derive(root, items, stack_problem):
    evidence = json.loads(migration.metadata_path(root, migration.EVIDENCE).read_text())
    packet = migration.derive_packet(root, evidence)
    migration.validate(packet, root)
    records = derive_definitions(packet, items, stack_problem)
    check = validator(root)
    for record in records:
        check.validate(record['definition'])
    return records


def validate(records, root, items, stack_problem):
    expected = derive(root, items, stack_problem)
    return [] if records == expected else ['canonical variant payload/source/hold inventory differs from exact source derivation']


def native_lowering_errors(records):
    """A compiler can use this boundary before attempting the plain native model."""
    return [{'claim': r['definition']['identity']['key'], 'code': 'REWARD_CLAIM_NATIVE_LOWERING_NOT_IMPLEMENTED',
             'pending': r['definition']['source_variant']['native_lowering']['pending_implementation']}
            for r in records if r['definition'].get('definition_profile') == PROFILE]


def extend_outputs(root, outputs, records, compact, content_dir):
    index_path = content_dir + 'index.json'; index = json.loads(outputs[index_path])
    plain_count = index['record_count']
    plain = [r for path in index['shards'] for r in json.loads(outputs[path])['records']]
    keys = [r['definition']['identity']['key'] for r in plain + records]
    positions = [tuple(p['source_binding']['project_position'][axis] for axis in ('x', 'y', 'z'))
                 for r in plain + records for p in r['definition']['placements']]
    if len(keys) != len(set(keys)) or len(positions) != len(set(positions)):
        raise ValueError('canonical plain/variant identity or placement collision')
    for shard_index, start in enumerate(range(0, len(records), 100)):
        chunk = records[start:start + 100]; end = start + len(chunk) - 1
        path = content_dir + f'reward-claim-variants-{start:05d}-{end:05d}.json'
        index['shards'].append(path)
        outputs[path] = compact({'family': 'RewardClaim', 'schema': SCHEMA,
            'definition_profile': PROFILE, 'native_admission': 'WAITING_IMPLEMENTATION',
            'shard': {'index': shard_index, 'start': start, 'end': end, 'count': len(chunk)}, 'records': chunk})
    index['record_count'] += len(records)
    index['plain_record_count'] = plain_count; index['variant_record_count'] = len(records)
    index['readiness'].update({status: sum(r['definition']['readiness'] == status for r in records)
                               for status in ('waiting_implementation', 'waiting_data')})
    index['native_admission'] = {'plain': 'EXISTING_ADMISSION', 'variants': 'WAITING_IMPLEMENTATION'}
    outputs[index_path] = compact(index)
