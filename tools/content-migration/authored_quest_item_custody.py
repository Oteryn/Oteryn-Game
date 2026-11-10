"""Native custody for explicit authored rewards with unknown canonical stack facts.

Reuse the pinned native importer for physical facts only. Canonical data and
accepted holds remain intact; weapon, equipment and use readiness are not inferred.
"""
import copy
import hashlib
import json

import grizzly_shop_profiles as grizzly
import quest_reward_item_semantics as reward


XML_SHA256 = '1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2'
CORRECTIONS = 'tools/content-schema/native-gameplay/quest-custody-weight-corrections.json'


def correct_weight(root, definition, node, row):
    """An explicit source conflict may retain only an unchanged accepted weight."""
    path = root / CORRECTIONS
    if not path.exists():
        return
    packet = json.loads(path.read_bytes())
    records = packet.get('records', [])
    if (set(packet) != {'schema', 'records'} or packet['schema'] != 1
            or not isinstance(records, list) or len(records) > 32
            or len({record['item'] for record in records}) != len(records)):
        raise ValueError('AUTHORED_CUSTODY_WEIGHT_CORRECTIONS')
    for record in records:
        if record['item'] != definition['identity']['key']:
            continue
        accepted = definition['semantics']['physical']['value']['weight']
        if (set(record) != {'item', 'canonical_sha256', 'source_xml_weight', 'weight', 'sources'}
                or hashlib.sha256(reward.canonical(definition)).hexdigest() != record['canonical_sha256']
                or grizzly.source.attrs(node).get('weight') != str(record['source_xml_weight'])
                or accepted != grizzly.source.K(record['weight'])
                or type(record['weight']) is not int or not 0 <= record['weight'] <= 0xffffffff
                or not record['sources'] or any(not isinstance(s, str) or not s for s in record['sources'])):
            raise ValueError('AUTHORED_CUSTODY_WEIGHT_CORRECTION_BINDING')
        row['semantics']['physical']['value']['weight'] = copy.deepcopy(accepted)


def _source(inputs, name, load):
    if inputs is None:
        return load()
    if name not in inputs:
        inputs[name] = load()
    return inputs[name]


def qualification_inputs(root, definitions, xml_raw):
    """One source snapshot for a bounded admission pass; callers never retain it."""
    if hashlib.sha256(xml_raw).hexdigest() != XML_SHA256:
        raise ValueError('AUTHORED_CUSTODY_XML_PIN')
    index = grizzly.source.binding_index((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    identities = {tuple(sorted(definition['identity'].items())) for definition in definitions}
    ids = {int(external) for external, binding in index.items()
           if tuple(sorted(binding['target'].items())) in identities and binding['disposition'] == 'EXACT'}
    return {'index': index, 'ids': ids}


def qualify_many(root, definitions, xml_raw, *, terminal_expiry=False):
    """Share immutable source reads within this pass, never across changed inputs."""
    definitions = list(definitions)
    if not definitions:
        return []
    inputs = qualification_inputs(root, definitions, xml_raw)
    return [qualify(root, definition, xml_raw, terminal_expiry=terminal_expiry, _inputs=inputs)
            for definition in definitions]


def qualify(root, definition, xml_raw, *, terminal_expiry=False, _inputs=None):
    if hashlib.sha256(xml_raw).hexdigest() != XML_SHA256:
        raise ValueError('AUTHORED_CUSTODY_XML_PIN')
    source = grizzly.source
    index = _source(_inputs, 'index', lambda: source.binding_index(
        (root / 'imports/crystalserver/bindings/items.json').read_bytes()))
    matches = [(external, binding) for external, binding in index.items()
               if binding['target'] == definition['identity'] and binding['disposition'] == 'EXACT']
    if len(matches) != 1:
        raise ValueError('AUTHORED_CUSTODY_EXACT_BINDING')
    item_id = int(matches[0][0])
    holds = _source(_inputs, 'admissions', lambda: reward.load_admissions(root)).get(
        definition['identity']['key'], ({},))[0].get('holds', [])
    if holds:
        raise ValueError('AUTHORED_CUSTODY_ACCEPTED_HOLD:' + ','.join(sorted(holds)))
    client = _source(_inputs, 'client', lambda: (root / reward.CLIENT).read_bytes())
    if hashlib.sha256(client).hexdigest() != reward.CLIENT_SHA or len(client) != reward.CLIENT_BYTES:
        raise ValueError('AUTHORED_CUSTODY_CLIENT_PIN')
    nodes = _source(_inputs, 'nodes', lambda: source.xml_index(xml_raw))
    node = nodes.get(item_id)
    if node is None:
        raise ValueError('AUTHORED_CUSTODY_SOURCE_ITEM')
    attributes = source.attrs(node)
    timed = {'charges', 'duration', 'decayto', 'stopduration'} & attributes.keys()
    if timed:
        if (not terminal_expiry or timed - {'duration', 'decayto'} or 'duration' not in timed
                or not 1 <= int(attributes['duration']) <= 0xffffffff // 1000
                or int(attributes.get('decayto', '0')) != 0):
            raise ValueError('AUTHORED_CUSTODY_INSTANCE_OWNER_REQUIRED')
    objects = _source(_inputs, 'objects', lambda: source.engine_items.load_appearance_objects(client))
    appearance = objects.get(item_id)
    if appearance is None or any(appearance['flags'].get('flags.' + name, False)
                                 for name in ('liquidcontainer', 'liquidpool')):
        raise ValueError('AUTHORED_CUSTODY_STATEFUL_APPEARANCE')
    policies = _source(_inputs, 'policies', lambda: {row['server_item_id']: row
        for row in source.policies(client, xml_raw, _inputs['ids'] if _inputs is not None else {item_id})})
    physical = policies.get(item_id)
    if physical is None:
        raise ValueError('AUTHORED_CUSTODY_PHYSICAL_SOURCE')
    row = source.profile(item_id, node, appearance, physical, index, None)
    correct_weight(root, definition, node, row)
    if (not row['admission']['materializable']
            or row['admission']['legal_destinations'] != ['CharacterInventory', 'Ground']):
        raise ValueError('AUTHORED_CUSTODY_NOT_PORTABLE')
    groups = ('physical', 'stack', 'container', 'charges', 'temporal') if timed else ('physical', 'stack', 'container')
    for group in groups:
        if grizzly.known_conflict(definition['semantics'][group], row['semantics'][group]):
            raise ValueError('AUTHORED_CUSTODY_CANONICAL_' + group.upper() + ':' + definition['identity']['key'])
    if definition['stack_class'] not in ('Unknown', row['admission']['stack_class']):
        raise ValueError('AUTHORED_CUSTODY_STACK_CONFLICT')
    # The importer may know source combat/use facts. This admission borrows only
    # its custody facts; all other groups remain exactly the accepted canonical data.
    for group in row['semantics']:
        row['semantics'][group] = (grizzly.accepted_first(definition['semantics'][group], row['semantics'][group])
                                   if group in groups
                                   else copy.deepcopy(definition['semantics'][group]))
    return source.bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': [row]},
                                index, {item_id})['records'][0]


def reuse_existing(definition, record, bindings):
    """Validate custody already qualified by an earlier native provider; change nothing.

    Stateful source Items still cannot enter through qualify(). Reusing a native
    admission preserves its existing timed/use owner and every accepted hold.
    """
    identity = definition['identity']
    expected = {'family': identity['family'], 'production_key': identity['key'],
                'revision_ref': identity['revision']}
    matches = [binding for binding in bindings if binding['target'] == identity]
    if (record['production_definition'] != expected or len(matches) != 1
            or matches[0]['disposition'] != 'EXACT'
            or record['production_binding'] != matches[0]):
        raise ValueError('AUTHORED_CUSTODY_EXISTING_IDENTITY')
    admission = record['admission']
    if (not admission['materializable']
            or admission['legal_destinations'] != ['CharacterInventory', 'Ground']
            or admission['stack_class'] not in ('NonStackable', 'StackCapable')):
        raise ValueError('AUTHORED_CUSTODY_EXISTING_ADMISSION')
    semantics = record['semantics']
    physical = semantics['physical'].get('value', {})
    weight = physical.get('weight', {})
    if (semantics['physical'].get('state') != 'KNOWN'
            or any(physical.get(field) != {'state': 'KNOWN', 'value': True}
                   for field in ('movable', 'pickupable'))
            or weight.get('state') != 'KNOWN' or type(weight.get('value')) is not int
            or not 0 <= weight['value'] <= 0xffffffff):
        raise ValueError('AUTHORED_CUSTODY_EXISTING_PHYSICAL')
    stack = semantics['stack']
    stackable = admission['stack_class'] == 'StackCapable'
    if (stack.get('state') != 'KNOWN'
            or stack.get('value', {}).get('stackable') != {'state': 'KNOWN', 'value': stackable}
            or (stackable and (stack['value']['stack_max'].get('state') != 'KNOWN'
                or type(stack['value']['stack_max'].get('value')) is not int
                or not 1 <= stack['value']['stack_max']['value'] <= 100))):
        raise ValueError('AUTHORED_CUSTODY_EXISTING_STACK')
    for group in ('physical', 'stack', 'container'):
        if grizzly.known_conflict(definition['semantics'][group], semantics[group]):
            raise ValueError('AUTHORED_CUSTODY_EXISTING_CANONICAL_' + group.upper())
