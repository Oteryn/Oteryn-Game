"""Native custody for explicit authored rewards with unknown canonical stack facts.

Reuse the pinned native importer for physical facts only. Canonical data and
accepted holds remain intact; weapon, equipment and use readiness are not inferred.
"""
import copy
import hashlib

import grizzly_shop_profiles as grizzly
import quest_reward_item_semantics as reward


XML_SHA256 = '1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2'


def qualify(root, definition, xml_raw):
    if hashlib.sha256(xml_raw).hexdigest() != XML_SHA256:
        raise ValueError('AUTHORED_CUSTODY_XML_PIN')
    source = grizzly.source
    index = source.binding_index((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    matches = [(external, binding) for external, binding in index.items()
               if binding['target'] == definition['identity'] and binding['disposition'] == 'EXACT']
    if len(matches) != 1:
        raise ValueError('AUTHORED_CUSTODY_EXACT_BINDING')
    item_id = int(matches[0][0])
    holds = reward.load_admissions(root).get(definition['identity']['key'], ({},))[0].get('holds', [])
    if holds:
        raise ValueError('AUTHORED_CUSTODY_ACCEPTED_HOLD:' + ','.join(sorted(holds)))
    client = (root / reward.CLIENT).read_bytes()
    if hashlib.sha256(client).hexdigest() != reward.CLIENT_SHA or len(client) != reward.CLIENT_BYTES:
        raise ValueError('AUTHORED_CUSTODY_CLIENT_PIN')
    nodes = source.xml_index(xml_raw)
    node = nodes.get(item_id)
    if node is None:
        raise ValueError('AUTHORED_CUSTODY_SOURCE_ITEM')
    attributes = source.attrs(node)
    if {'charges', 'duration', 'decayto', 'stopduration'} & attributes.keys():
        raise ValueError('AUTHORED_CUSTODY_INSTANCE_OWNER_REQUIRED')
    objects = source.engine_items.load_appearance_objects(client)
    appearance = objects.get(item_id)
    if appearance is None or any(appearance['flags'].get('flags.' + name, False)
                                 for name in ('liquidcontainer', 'liquidpool')):
        raise ValueError('AUTHORED_CUSTODY_STATEFUL_APPEARANCE')
    physical = next((row for row in source.policies(client, xml_raw, {item_id})
                     if row['server_item_id'] == item_id), None)
    if physical is None:
        raise ValueError('AUTHORED_CUSTODY_PHYSICAL_SOURCE')
    row = source.profile(item_id, node, appearance, physical, index, None)
    if (not row['admission']['materializable']
            or row['admission']['legal_destinations'] != ['CharacterInventory', 'Ground']):
        raise ValueError('AUTHORED_CUSTODY_NOT_PORTABLE')
    for group in ('physical', 'stack', 'container'):
        if grizzly.known_conflict(definition['semantics'][group], row['semantics'][group]):
            raise ValueError('AUTHORED_CUSTODY_CANONICAL_' + group.upper())
    if definition['stack_class'] not in ('Unknown', row['admission']['stack_class']):
        raise ValueError('AUTHORED_CUSTODY_STACK_CONFLICT')
    # The importer may know source combat/use facts. This admission borrows only
    # its custody facts; all other groups remain exactly the accepted canonical data.
    for group in row['semantics']:
        row['semantics'][group] = (grizzly.accepted_first(definition['semantics'][group], row['semantics'][group])
                                   if group in ('physical', 'stack', 'container')
                                   else copy.deepcopy(definition['semantics'][group]))
    return source.bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': [row]},
                                index, {item_id})['records'][0]
