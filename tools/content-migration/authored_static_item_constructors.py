"""Explicit first-party static constructors; never a donor server-ID promotion.

Only portable, nonstacking canonical Items with matching pinned client facts and
no accepted holds qualify. Noncustody semantics and original definitions remain
unchanged. The generated admission packet binds every byte of each native row.
"""
import copy
from functools import lru_cache
import hashlib
import json

from native_profile_composition import encode_native_profile
from pathlib import Path

import grizzly_shop_profiles as grizzly
import quest_reward_item_semantics as reward

INTENT = 'tools/content-schema/native-gameplay/authored-static-item-constructors.json'
ADMISSIONS = 'content/items/authored-static-admissions.json'
SOURCE = 'oteryn:source.oteryn_authored'
NAMESPACE = 'oteryn/static_item_constructor'
SCHEMA = 'OTERYN_AUTHORED_STATIC_ITEM_ADMISSIONS/v1'


def canonical(value):
    return json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(',', ':')).encode()


def digest(value):
    return hashlib.sha256(canonical(value)).hexdigest()


@lru_cache(maxsize=2)
def definitions(root):
    result = {}
    for path in sorted((root / 'content/items/definitions').glob('items-*.json')):
        for row in json.loads(path.read_bytes())['records']:
            definition = row['definition']
            result[definition['identity']['key']] = definition
    return result


@lru_cache(maxsize=2)
def quest_definitions(root):
    result = {}
    for path in sorted((root / 'content/quests/definitions').glob('quests-*.json')):
        for record in json.loads(path.read_bytes())['records']:
            definition = record['definition']
            result[definition['identity']['key']] = definition
    return result


@lru_cache(maxsize=2)
def facts(root):
    client = (root / reward.CLIENT).read_bytes()
    if hashlib.sha256(client).hexdigest() != reward.CLIENT_SHA or len(client) != reward.CLIENT_BYTES:
        raise ValueError('AUTHORED_STATIC_CLIENT_PIN')
    appearances = grizzly.source.engine_items.load_appearance_objects(client)
    bindings = json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())['bindings']
    return appearances, bindings, reward.load_admissions(root)


def prepare(root, intent):
    if (intent.get('schema') != 'OTERYN_AUTHORED_STATIC_ITEM_CONSTRUCTORS/v1'
            or intent.get('basis') != 'CHOSEN_OTERYN_APPROXIMATION'
            or not 1 <= len(intent.get('constructors', [])) <= 64):
        raise ValueError('AUTHORED_STATIC_INTENT')
    recipe_path = Path(intent['recipes']['path'])
    if recipe_path.is_absolute() or '..' in recipe_path.parts:
        raise ValueError('AUTHORED_STATIC_RECIPE_PATH')
    raw = (root / recipe_path).read_bytes()
    if hashlib.sha256(raw).hexdigest() != intent['recipes']['sha256']:
        raise ValueError('AUTHORED_STATIC_RECIPE_PIN')
    recipes = json.loads(raw)['recipes']
    appearances, bindings, accepted = facts(root)
    profiles, admitted, seen = [], [], set()
    intent_hash = digest(intent)
    for declaration in intent['constructors']:
        key, quest, weight = declaration['item'], declaration['quest'], declaration['weight']
        if (key in seen or not key.startswith('oteryn:item.tibia.i')
                or not quest.startswith('oteryn:quest.authored.')
                or type(weight) is not int or not 1 <= weight <= 0xffffffff):
            raise ValueError('AUTHORED_STATIC_IDENTITY_OR_WEIGHT')
        seen.add(key)
        definition = definitions(root).get(key)
        if definition is None:
            raise ValueError('AUTHORED_STATIC_CANONICAL_ITEM')
        identifier = int(key.removeprefix('oteryn:item.tibia.i'))
        if str(identifier) != key.removeprefix('oteryn:item.tibia.i') or identifier < 1:
            raise ValueError('AUTHORED_STATIC_NUMERIC_IDENTITY')
        if any(row['target'] == definition['identity'] or row['external_id'] == str(identifier) for row in bindings):
            raise ValueError('AUTHORED_STATIC_EXISTING_DONOR_BINDING')
        matches = [recipe for recipe in recipes if recipe['wiki_title'] == declaration['recipe_title']]
        if len(matches) != 1:
            raise ValueError('AUTHORED_STATIC_RECIPE')
        recipe = matches[0]
        quest_definition = quest_definitions(root).get(quest)
        if (quest_definition is None
                or quest_definition.get('display_name') != declaration['recipe_title']
                or quest_definition.get('provenance') != intent['recipes']):
            raise ValueError('AUTHORED_STATIC_QUEST_RECIPE_BINDING')
        roles = {target for stage in recipe['stages'] for target in stage['targets']}
        roles.update(reward['name'] for reward in recipe['reward_intents'])
        if declaration['role'] not in roles:
            raise ValueError('AUTHORED_STATIC_ROLE')
        if accepted.get(key, ({},))[0].get('holds'):
            raise ValueError('AUTHORED_STATIC_ACCEPTED_HOLD')
        semantics = definition['semantics']
        known = lambda value: {'state': 'KNOWN', 'value': value}
        physical = semantics['physical']
        stack = semantics['stack']
        if (physical.get('state') != 'KNOWN'
                or any(physical['value'].get(field) != known(True) for field in ('movable', 'pickupable'))
                or stack.get('state') != 'KNOWN' or stack['value'].get('stackable') != known(False)
                or semantics['charges'].get('state') == 'KNOWN'
                or semantics['temporal'].get('state') == 'KNOWN'):
            raise ValueError('AUTHORED_STATIC_CANONICAL_CUSTODY')
        current_weight = physical['value']['weight']
        if current_weight.get('state') == 'KNOWN' and current_weight != known(weight):
            raise ValueError('AUTHORED_STATIC_KNOWN_WEIGHT')
        appearance = appearances.get(identifier)
        flags = appearance['flags'] if appearance else {}
        if (appearance is None or not flags.get('flags.take', False)
                or any(flags.get('flags.' + name, False) for name in
                       ('unmove', 'cumulative', 'container', 'liquidcontainer', 'liquidpool', 'clothes', 'expire', 'wearout'))):
            raise ValueError('AUTHORED_STATIC_CLIENT_CUSTODY')
        canonical_name = semantics['presentation'].get('value', {}).get('name', {})
        if canonical_name != known(appearance.get('name')):
            raise ValueError('AUTHORED_STATIC_CLIENT_NAME')
        result_semantics = copy.deepcopy(semantics)
        result_semantics['physical'] = known({'weight': known(weight), 'movable': known(True), 'pickupable': known(True)})
        result_semantics['stack'] = known({'stackable': known(False), 'stack_max': {'state': 'NOT_APPLICABLE'}})
        container = {'state': 'NOT_APPLICABLE'}
        if grizzly.known_conflict(semantics['container'], container):
            raise ValueError('AUTHORED_STATIC_CANONICAL_CONTAINER')
        result_semantics['container'] = container
        identity = definition['identity']
        binding = {'source_key': SOURCE, 'source_revision': intent_hash,
                   'identity_namespace': NAMESPACE, 'external_id': str(identifier),
                   'target': copy.deepcopy(identity), 'disposition': 'EXACT'}
        row = {
            'authoring': {'item': copy.deepcopy(identity)},
            'semantics': result_semantics,
            'attributes': {'speed_bonus': 0, 'blocks_movement': flags.get('flags.unpass', False),
                           'blocks_projectile': flags.get('flags.unsight', False),
                           'immovable_block_solid': False, 'has_height': flags.get('flags.height', False),
                           'field_replaceable': None, 'field_condition': None, 'rune_consumption': None},
            'admission': {'materializable': True, 'stack_class': 'NonStackable',
                          'legal_destinations': ['CharacterInventory', 'Ground']},
            'production_definition': {'family': 'Item', 'production_key': key, 'revision_ref': identity['revision']},
            'production_binding': binding,
            'production_binding_qualification': {
                'schema': 'OTERYN_FIRST_PARTY_STATIC_CONSTRUCTOR/v1', 'basis': intent['basis'],
                'intent_sha256': intent_hash, 'canonical_definition_sha256': digest(definition),
                'client_sha256': reward.CLIENT_SHA, 'recipe_sha256': intent['recipes']['sha256'],
                'quest': quest, 'role': declaration['role'],
            },
        }
        profiles.append(row)
        admitted.append({'binding': copy.deepcopy(binding), 'profile_sha256': digest(row)})
    return profiles, {'schema': SCHEMA, 'records': admitted}


def matches_admission(profile, admissions):
    return (admissions.get('schema') == SCHEMA and any(
        row['binding'] == profile['production_binding'] and row['profile_sha256'] == digest(profile)
        for row in admissions['records']))


def admit(root, generated, profile_path, manifest_path, order, encode):
    profiles, admissions = prepare(root, json.loads((root / INTENT).read_bytes()))
    document = json.loads(generated[profile_path])
    present = {row['production_definition']['production_key'] for row in document['records']}
    if present & {row['production_definition']['production_key'] for row in profiles}:
        raise ValueError('AUTHORED_STATIC_DUPLICATE_PROFILE')
    document['records'] = sorted(document['records'] + profiles, key=order)
    data = encode_native_profile(document)
    generated[profile_path] = data
    generated[ADMISSIONS] = (json.dumps(admissions, indent=2, ensure_ascii=False) + '\n').encode()
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(data).hexdigest()
    generated[manifest_path] = encode(manifest)
