"""Carry already admitted quest offering and chest definitions into the native Item provider."""
import copy
import hashlib
import json
import xml.etree.ElementTree as ET

import writable_item_profiles as writing

SOURCE = 'imports/canary/items-xml/items.xml'
SOURCE_SHA256 = '1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2'


def complete_native_stack(record, definition, binding):
    """Fill only unqualified stack facts; established native facts keep authority."""
    identity = definition['identity']
    expected = {'family': identity['family'], 'production_key': identity['key'],
                'revision_ref': identity['revision']}
    if record['production_definition'] != expected or record['production_binding'] != binding:
        raise ValueError('QUEST_CALLBACK_NATIVE_IDENTITY')
    admission = record['admission']
    if (not admission['materializable']
            or 'CharacterInventory' not in admission['legal_destinations']
            or admission['stack_class'] != definition['stack_class']):
        raise ValueError('QUEST_CALLBACK_NATIVE_ITEM_NOT_ADMITTED')
    semantics = record['semantics']
    physical = semantics['physical']
    weight = physical.get('value', {}).get('weight', {})
    if (physical.get('state') != 'KNOWN' or weight.get('state') != 'KNOWN'
            or type(weight.get('value')) is not int or not 0 <= weight['value'] <= 0xffffffff
            or any(physical['value'].get(field) != {'state': 'KNOWN', 'value': True}
                   for field in ('movable', 'pickupable'))):
        raise ValueError('QUEST_CALLBACK_NATIVE_PHYSICAL_INCOMPLETE')
    if admission['stack_class'] == 'StackCapable':
        observed, accepted = semantics['stack'], definition['semantics']['stack']
        if observed.get('state') == 'UNKNOWN' and accepted.get('state') == 'KNOWN':
            semantics['stack'] = copy.deepcopy(accepted)
        elif observed.get('state') == accepted.get('state') == 'KNOWN':
            for field in ('stackable', 'stack_max'):
                if (observed['value'][field].get('state') == 'UNKNOWN'
                        and accepted['value'][field].get('state') == 'KNOWN'):
                    observed['value'][field] = copy.deepcopy(accepted['value'][field])
        stack = semantics['stack']
        maximum = stack.get('value', {}).get('stack_max', {})
        if (stack.get('state') != 'KNOWN'
                or stack['value'].get('stackable') != {'state': 'KNOWN', 'value': True}
                or maximum.get('state') != 'KNOWN' or type(maximum.get('value')) is not int
                or not 1 <= maximum['value'] <= 100):
            raise ValueError('QUEST_CALLBACK_NATIVE_STACK_INCOMPLETE')
    expected_container = definition['semantics']['container']
    if expected_container.get('state') == 'KNOWN':
        expected_capacity = expected_container['value']['capacity']
        if expected_capacity.get('state') == 'KNOWN':
            container = semantics['container']
            if (container.get('state') != 'KNOWN'
                    or container['value']['capacity'] != expected_capacity):
                raise ValueError('QUEST_CALLBACK_NATIVE_CONTAINER_INCOMPLETE')


def grant_semantics(root, definition):
    """Carry source text facts without inventing an unqualified player-write policy."""
    semantics = copy.deepcopy(definition['semantics'])
    text = semantics['readable_writeable']
    if (text.get('state') != 'KNOWN'
            or text['value']['writeable'] != writing.known(True)):
        return semantics
    maximum = text['value']['max_text_length']
    if (maximum.get('state') == 'KNOWN' and type(maximum.get('value')) is int
            and 1 <= maximum['value'] <= writing.MAX_CHARACTERS):
        return semantics
    # A source-writable sheet with no qualified limit is still a valid authored
    # quest-text carrier (PR2770 Q13). The existing WRITE owner classifies it as
    # READ_ONLY_OR_ZERO_LIMIT. Reuse that pinned qualification: retain the
    # canonical source facts and grant admission, but assert no player WRITE
    # authority and invent neither a text limit nor a write-once target.
    expected = writing.outputs(root)[writing.PATH]
    if (root / writing.PATH).read_bytes() != expected:
        raise ValueError('QUEST_CALLBACK_WRITABLE_PROFILE_DRIFT')
    profile = next((row for row in json.loads(expected)['records']
                    if row['item'] == definition['identity']), None)
    if (profile is None or profile['qualification'] != 'READ_ONLY_OR_ZERO_LIMIT'
            or profile['max_characters'] != 0 or profile['writable'] is not True
            or profile['readable'] is not True or profile['write_once_to'] is not None
            or maximum != {'state': 'UNKNOWN'}
            or text['value']['readable'] != writing.known(True)
            or text['value']['write_once_target'] != {'state': 'UNKNOWN'}):
        raise ValueError('QUEST_CALLBACK_WRITABLE_GRANT_UNQUALIFIED')
    writing.owner_replace(text['value'], 'writeable', {'state': 'UNKNOWN'},
                          [writing.known(True)])
    return semantics


def admit(root, generated, profile_path, manifest_path, order, canonical):
    callbacks = json.loads((root / 'content/quests/world-callbacks.json').read_bytes())
    keys = {offering['item'] for callback in callbacks['callbacks']
            for offering in callback['offerings']}
    for callback in callbacks['callbacks']:
        item = callback.get('item_on_target')
        if item:
            keys.add(item['item'])
            if item.get('replacement'):
                keys.add(item['replacement'])
    # These source chests also advance a quest track. Their Item definitions
    # already have canonical admission; retain the same proof in the native provider.
    for path in sorted((root / 'content/interactions/reward_claims').glob('reward-claim*.json')):
        for row in json.loads(path.read_bytes())['records']:
            definition = row['definition']
            # W15b's complete recipe already qualified the canonical root and
            # every child. Carry that whole closure into the native provider;
            # admitting only the visible chest reward would mint a partial bag.
            for placement in definition.get('native_contents', {}).get('placements', []):
                keys.add(placement['root']['key'])
                keys.update(child['item']['key'] for child in placement['contents'])
            if definition.get('readiness') == 'ready':
                for placement in definition['placements']:
                    if placement.get('quest_transition'):
                        keys.update(item['item']['key'] for item in placement['reward']['items'])
    grizzly_path = root / 'content/quests/killing-in-the-name-of.json'
    if grizzly_path.is_file():
        grizzly = json.loads(grizzly_path.read_bytes())
        # The fixed Holy Icon entitlement uses its existing canonical identity
        # and admission. Shop Items with unqualified facts are a separate input.
        keys.update(f"oteryn:item.tibia.i{task['item']['source_id']}"
                    for task in grizzly['tasks'] if task.get('item'))
    document = json.loads(generated[profile_path])
    present = {row['production_definition']['production_key']: row for row in document['records']}
    if len(present) != len(document['records']):
        raise ValueError('QUEST_CALLBACK_DUPLICATE_NATIVE_ITEM')
    definitions = {}
    for path in sorted((root / 'content/items/definitions').glob('items-*.json')):
        for row in json.loads(path.read_bytes())['records']:
            definition = row['definition']
            key = definition['identity']['key']
            if key in keys:
                if key in definitions:
                    raise ValueError('QUEST_CALLBACK_DUPLICATE_ITEM')
                definitions[key] = definition
    if set(definitions) != keys:
        raise ValueError('QUEST_CALLBACK_ITEM_MISSING')
    bindings = json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())['bindings']
    raw = (root / SOURCE).read_bytes()
    if hashlib.sha256(raw).hexdigest() != SOURCE_SHA256:
        raise ValueError('QUEST_CALLBACK_SOURCE_DIGEST')
    source_items = ET.fromstring(raw).findall('item')
    for key, definition in sorted(definitions.items()):
        identity = definition['identity']
        matches = [binding for binding in bindings if binding['target'] == identity]
        if (not definition['materializable']
                or definition['stack_class'] not in ('StackCapable', 'NonStackable')
                or len(matches) != 1 or matches[0]['disposition'] != 'EXACT'):
            raise ValueError('QUEST_CALLBACK_ITEM_NOT_ADMITTED')
        physical = definition['semantics']['physical']
        if (physical['state'] != 'KNOWN'
                or any(physical['value'][field] != {'state': 'KNOWN', 'value': True}
                       for field in ('movable', 'pickupable'))):
            raise ValueError('QUEST_CALLBACK_ITEM_NOT_PORTABLE')
        if key in present:
            complete_native_stack(present[key], definition, matches[0])
            continue
        item_id = int(matches[0]['external_id'])
        source_matches = [item for item in source_items
                          if int(item.get('id', item.get('fromid', '0'))) <= item_id
                          <= int(item.get('id', item.get('toid', '0')))]
        if len(source_matches) != 1:
            raise ValueError('QUEST_CALLBACK_SOURCE_ITEM_MISSING')
        speeds = [attr for attr in source_matches[0].findall('attribute')
                  if attr.get('key', '').lower() == 'speed']
        if len(speeds) > 1:
            raise ValueError('QUEST_CALLBACK_DUPLICATE_SPEED')
        # Canary's Abilities constructor defaults speed to zero when XML omits it.
        speed = int(speeds[0].attrib['value']) if speeds else 0
        document['records'].append({
            'authoring': {'item': identity},
            'semantics': grant_semantics(root, definition),
            'attributes': {'speed_bonus': speed},
            'admission': {'materializable': True, 'stack_class': definition['stack_class'],
                          'legal_destinations': ['CharacterInventory', 'Ground']},
            'production_definition': {'family': identity['family'], 'production_key': key,
                                      'revision_ref': identity['revision']},
            'production_binding': matches[0], 'production_binding_qualification': None,
        })
        complete_native_stack(document['records'][-1], definition, matches[0])
    document['records'].sort(key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)
