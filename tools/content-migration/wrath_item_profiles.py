"""Wrath pillar and ground-only remains from the pinned source Item facts."""
import copy
import hashlib
import json
import xml.etree.ElementTree as ET

import native_goal_corpse_items as corpse
import primal_corpse_items as primal

PILLAR = 10797
IDS = (PILLAR, 11361, 11429)
XML = 'imports/canary/items-xml/items.xml'


def admit(root, generated, profile_path, manifest_path, order, canonical):
    keys = {f'oteryn:item.tibia.i{item}' for item in IDS}
    objects = corpse.world_objects(root, keys)
    xml = ET.fromstring((root / XML).read_bytes())
    shards = ('items-00500-00999.json', 'items-01000-01499.json')
    definitions = {
        row['definition']['identity']['key']: row['definition']
        for shard in shards
        for row in json.loads((root / 'content/items/definitions' / shard).read_bytes())['records']
        if row['definition']['identity']['key'] in keys
    }
    bindings = json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())['bindings']
    document = json.loads(generated[profile_path])
    for item in IDS:
        key = f'oteryn:item.tibia.i{item}'
        definition, world = definitions[key], objects[key]
        identity = definition['identity']
        exact = [b for b in bindings if b['target'] == identity and b['disposition'] == 'EXACT']
        remains = item != PILLAR
        expected_xml = {'duration': '60', 'decayto': '0'} if remains else {}
        nodes = xml.findall(f"item[@id='{item}']")
        if (len(exact) != 1 or len(nodes) != 1 or world['provenance']['item_pointer'] != identity
                or world['movable'] != {'state': 'KNOWN', 'value': False}
                or {a.get('key').lower(): a.get('value') for a in nodes[0]}
                != expected_xml):
            raise ValueError('Wrath remains source identity, mobility or lifetime')
        if any(r['production_definition']['production_key'] == key for r in document['records']):
            raise ValueError('Wrath remains duplicate native profile')
        observation = primal.corpse_definition(definition, None, 60_000, None, False)
        semantics = {**copy.deepcopy(observation['semantics']),
                     **copy.deepcopy(definition.get('semantics', {}))}
        groups = corpse.GROUPS if remains else ('physical', 'stack', 'container')
        if not remains and 'temporal' not in definition.get('semantics', {}):
            semantics['temporal'] = {'state': 'UNKNOWN'}
        for group in groups:
            if not corpse.compatible(semantics[group], observation['semantics'][group]):
                raise ValueError(f'Wrath remains canonical {group} conflict: {key}')
            semantics[group] = observation['semantics'][group]
        record = dict(authoring=dict(item=identity), semantics=semantics,
                      attributes=dict(speed_bonus=0),
                      admission=dict(materializable=remains, stack_class='NonStackable', legal_destinations=['Ground']),
                      production_definition=dict(family='Item', production_key=key, revision_ref=identity['revision']),
                      production_binding=exact[0], production_binding_qualification=None)
        classification, collision = corpse.source_facts(record, world, nodes[0])
        if not corpse.compatible(semantics['classification'], classification):
            raise ValueError(f'Wrath remains classification conflict: {key}')
        semantics['classification'] = classification
        record['attributes'].update(collision)
        document['records'].append(record)
    document['records'].sort(key=order)
    raw = canonical(document)
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)
