"""Remaining ITEMHOLD-Q admissions, under the recorded Wiki-over-OTS policy.

Existing nine admissions are untouched. Conflicting donor weights remain witnesses;
only the exact Wiki revisions below supply the selected physical values.
"""
import copy
import hashlib
import json
import sys
import xml.etree.ElementTree as ET
from decimal import Decimal

IDS = (33330, 44179, 52732)
RETAINED = 'imports/ots-source-evidence/item-fx-audio295/retained-source-files.json'
CRYSTAL_REVISION = 'ff7ede593c69d4c658b382c97443e8155926924a'
CRYSTAL_SHA = 'c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb'
WIKI = 'imports/tibiawiki/facts/items-stats.json'
WITNESSES = {
    33330: (92100, 1013917, 'dc578d743ac45e14f7e8bd11c9bf1fd8c325f00349da29a47075d31ce20f940b', 7500),
    44179: (103946, 1041565, 'eb5189ea4ef782741a3b67bd2794c5e5e72f2f6ed7bc86ad553cba1ccd9ac929', 60),
    52732: (109358, 1194069, '76eb0c1ddeaf696f4c94a4cf5cd81b7e2a20b35aac65675f56a3210636023a31', 90),
}
NA = {'state': 'NOT_APPLICABLE'}


def known(value):
    return {'state': 'KNOWN', 'value': value}


def bottle_binding():
    # Own numeric itemid + exact 15.30 appearance, not a fabricated Crystal row.
    return {'source_key': 'oteryn:source.tibiawiki', 'source_revision': 'fandom-r1041565',
            'identity_namespace': 'tibiawiki/item_id', 'external_id': '44179',
            'target': {'family': 'Item', 'key': 'oteryn:item.tibia.i44179', 'revision': 'definition-r1'},
            'disposition': 'EXACT'}


def source_facts(root):
    import quest_reward_item_semantics as reward
    raw = (root / reward.CLIENT).read_bytes()
    if hashlib.sha256(raw).hexdigest() != reward.CLIENT_SHA:
        raise ValueError('ITEMHOLD_Q_REMAINING_CLIENT_PIN')
    sys.path.insert(0, str(root / 'tools/content-schema/item-authoring'))
    from engine_items import load_appearance_objects
    objects = load_appearance_objects(raw)
    files = json.loads((root / RETAINED).read_bytes())['source_files']
    defaults, = [r for r in files if r['repository'] == 'zimbadev/crystalserver'
                 and r['repository_path'] == 'src/items/items.hpp' and r['revision'] == CRYSTAL_REVISION]
    defaults_sha = '4bb45e54fc06921e82b12badee389330376f72f352b2ef8c0074e824cf7b9844'
    if (defaults['sha256'] != defaults_sha
            or hashlib.sha256(defaults['raw_utf8'].encode()).hexdigest() != defaults_sha
            or any(token not in defaults['raw_utf8'] for token in
                   ('int32_t speed = 0;', 'uint32_t decayTime = 0;', 'uint32_t charges = 0;'))):
        raise ValueError('ITEMHOLD_Q_REMAINING_CONSTRUCTOR_PIN')
    donor, = [r for r in files if r['repository'] == 'zimbadev/crystalserver'
              and r['repository_path'] == 'data/items/items.xml' and r['revision'] == CRYSTAL_REVISION]
    if donor['sha256'] != CRYSTAL_SHA or hashlib.sha256(donor['raw_utf8'].encode()).hexdigest() != CRYSTAL_SHA:
        raise ValueError('ITEMHOLD_Q_REMAINING_CRYSTAL_PIN')
    nodes = {int(n.get('id')): n for n in ET.fromstring(donor['raw_utf8']) if n.get('id')}
    wiki = json.loads((root / WIKI).read_bytes())['records']
    own = json.loads((root / 'imports/tibiawiki/source-evidence/stack-default-successor8/global-own-itemid-index.json').read_bytes())
    result = {}
    for item, (page, revision, digest, weight) in WITNESSES.items():
        row = wiki[f'oteryn:item.tibia.i{item}']
        observation, = row['observations']
        if (row['item_id'] != item or observation['page_id'] != page
                or observation['revision_id'] != revision or observation['content_sha256'] != digest
                or Decimal(observation['fields']['weight']) * 100 != weight):
            raise ValueError('ITEMHOLD_Q_REMAINING_WIKI_PIN')
        mentions = own['by_own_itemid_integer_mention'][str(item)]
        if not any(m['page_id'] == page and m['revision_id'] == revision
                   and m['content_sha256'] == digest and m['raw_itemid'] == str(item)
                   and m['strict_numeric_itemid'] and m['balanced'] and not m['inside_comment']
                   and not m['duplicate_itemid_parameters'] for m in mentions):
            raise ValueError('ITEMHOLD_Q_REMAINING_OWN_ID')
        obj = objects[item]; flags = obj['flags']
        if (not flags.get('flags.take') or any(flags.get(f, False) for f in
                ('flags.cumulative', 'flags.container', 'flags.unmove', 'flags.liquidcontainer', 'flags.liquidpool',
                 'flags.unpass', 'flags.unsight')) or bool(flags.get('flags.height')) != (item == 33330)):
            raise ValueError('ITEMHOLD_Q_REMAINING_FLAGS')
        if item != 44179:
            node = nodes[item]
            attrs = {a.get('key', '').lower(): a.get('value') for a in node.findall('attribute')}
            if (node.get('name') != obj['name'] or attrs.get('weight') != ('750' if item == 33330 else '90')
                    or any(k in attrs for k in ('charges', 'duration', 'decayto', 'containersize'))):
                raise ValueError('ITEMHOLD_Q_REMAINING_XML_FACTS')
            if item == 52732 and (attrs.get('armor') != '1' or observation['fields'].get('levelrequired') != '0'):
                raise ValueError('ITEMHOLD_Q_GUARDIAN_FACTS')
        elif (obj['name'] != 'bottled dragon breath' or item in nodes
              or flags.get('light.brightness') != 2 or flags.get('light.color') != 186):
            raise ValueError('ITEMHOLD_Q_BOTTLE_APPEARANCE')
        result[item] = weight
    return result


def apply_admissions(definitions, root):
    import quest_reward_item_semantics as reward
    facts = source_facts(root)
    for definition in definitions:
        key = definition['identity']['key']
        if key not in {f'oteryn:item.tibia.i{i}' for i in IDS}:
            continue
        item = int(key.rsplit('.i', 1)[1])
        if definition['identity'] != {'family': 'Item', 'key': key, 'revision': 'definition-r1'}:
            raise ValueError('ITEMHOLD_Q_REMAINING_IDENTITY')
        if definition['stack_class'] not in ('Unknown', 'NonStackable'):
            raise ValueError('ITEMHOLD_Q_REMAINING_STACK_CONFLICT')
        # Wiki is authoritative for the atomic weight field; preserve conflicting XML above.
        reward.put_group(definition, 'physical', {'weight': known(facts[item]),
                         'movable': known(True), 'pickupable': known(True)})
        reward.put_group(definition, 'stack', {'stackable': known(False), 'stack_max': NA})
        for group in ('charges', 'temporal', 'container'):
            if definition['semantics'].get(group, reward.UNKNOWN) not in (reward.UNKNOWN, NA):
                raise ValueError('ITEMHOLD_Q_REMAINING_MECHANICS_CONFLICT')
            definition['semantics'][group] = copy.deepcopy(NA)
        if item == 52732:
            pattern, = definition['semantics']['equipment']['value']['patterns']['value']
            if pattern['primary_slot'] != known('RING'):
                raise ValueError('ITEMHOLD_Q_GUARDIAN_SLOT')
            for field, value in {'level': known(0), 'vocations': known([]), 'compatibility_rule': NA}.items():
                if pattern[field] not in (reward.UNKNOWN, value):
                    raise ValueError('ITEMHOLD_Q_GUARDIAN_REQUIREMENT_CONFLICT')
                pattern[field] = copy.deepcopy(value)
        if item == 44179:
            for group in ('equipment', 'fluid'):
                if definition['semantics'].get(group, reward.UNKNOWN) not in (reward.UNKNOWN, NA):
                    raise ValueError('ITEMHOLD_Q_BOTTLE_KIND_CONFLICT')
                definition['semantics'][group] = copy.deepcopy(NA)
        definition['stack_class'] = 'NonStackable'
        definition['materializable'] = True


def admit(root, generated, profile_path, manifest_path, order, canonical):
    source_facts(root)
    document = json.loads(generated[profile_path])
    bindings = json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())['bindings']
    for path in sorted((root / 'content/items/definitions').glob('items-*.json')):
        for row in json.loads(path.read_bytes())['records']:
            definition = row['definition']; key = definition['identity']['key']
            if key not in {f'oteryn:item.tibia.i{i}' for i in IDS}:
                continue
            binding = bottle_binding() if key.endswith('.i44179') else next(b for b in bindings if b['target'] == definition['identity'])
            if (binding['disposition'] != 'EXACT' or not definition['materializable']
                    or any(r['production_definition']['production_key'] == key for r in document['records'])):
                raise ValueError('ITEMHOLD_Q_REMAINING_NATIVE_CONFLICT')
            document['records'].append({
                'authoring': {'item': definition['identity']}, 'semantics': copy.deepcopy(definition['semantics']),
                'attributes': {'speed_bonus': 0, 'blocks_movement': False, 'blocks_projectile': False,
                               'immovable_block_solid': False, 'has_height': key.endswith('.i33330')},
                'admission': {'materializable': True, 'stack_class': 'NonStackable',
                              'legal_destinations': ['CharacterInventory', 'Ground']},
                'production_definition': {'family': 'Item', 'production_key': key, 'revision_ref': 'definition-r1'},
                'production_binding': binding, 'production_binding_qualification': None})
    document['records'].sort(key=order)
    raw = canonical(document); generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)
