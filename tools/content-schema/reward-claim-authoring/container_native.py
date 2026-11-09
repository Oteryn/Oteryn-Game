"""Lower complete canonical container variants against the pinned Global handler.

Source dispositions remain intact. `native_contents` is a separate, reproducible
admission of the exact root and every child; a missing child never becomes an
empty or partial reward. Source conflicts and other reward formats stay held.
"""
import copy
import hashlib
import json
import re

from native_container_fluids import project_fluid_child
from pathlib import Path

SOURCE = Path(__file__).with_name('container_native_source.json')
SOURCE_SHA256 = 'b073d546930a60adf511677f932e6e0d37df3a230313dc85c4b9253742325757'
REVISION = '201189f871d23597b82caf88fd611454a2ddefb8'
# Closed publication scope: other lanes retain their source claim ownership.
FLUID_QUEST_CLAIMS = {'canary:reward-claim/quest/u7_9/dreamers_challenge/chests_wine'}
MAX_CONTENTS = 20  # Existing BAGS-0 direct-container admission, not a donor capacity.
# Existing selected owner rulings, reaffirmed by PR2770 ANSWER 6542716086/Q4.
# Their source total remains 200; this deliberately preserves the accepted
# native two-stack recipe over Container:addItem's donor clamp to one stack.
ACCEPTED_TWO_STACKS = {6218: 3031, 6226: 3450}
# These Item profiles already have native charged-instance custody. The
# historical canonical hold predates that owner. Scope the admission to these
# exact quest claims; neither global Item definitions nor other quests change.
CHARGED_QUEST_CLAIMS = {
    'canary:reward-claim/kv/desert2': 3084,
    'canary:reward-claim/kv/fireaxe1': 3085,
    'canary:reward-claim/kv/draconia1': 3284,
    'canary:reward-claim/kv/timering3': 3082,
    'canary:reward-claim/kv/triangletower': 3083,
    'canary:reward-claim/kv/trollcave1': 3083,
    'canary:reward-claim/kv/weddingring1': 3085,
}
FULL_CHARGE_LEAVES = {
    'canary:reward-claim/kv/madmageroom3': (6082, 3081, 5),
    'canary:reward-claim/kv/timering3': (6051, 3082, 50),
    'canary:reward-claim/kv/trollcave1': (6039, 3083, 150),
    'canary:reward-claim/kv/weddingring1': (6043, 3085, 200),
}
CHARGE_SOURCE_SHA256 = 'b1c6603253e04042c113a8e5c69bff2b58b4f61f94f03ffdf4df6a07906c7dec'


def native_charge_item(item, profile):
    """Use existing native custody only when canonical physical/custody facts agree."""
    if item is None or profile is None:
        return item
    identity = item['identity']
    key = identity['key']
    charges = {3082: 50, 3083: 150, 3084: 250, 3085: 200, 3284: 1}.get(item_id(identity))
    semantics = profile['semantics']
    binding = profile['production_binding']
    if (charges is None or profile['production_definition'] != {
            'family': 'Item', 'production_key': key, 'revision_ref': identity['revision']}
            or binding['target'] != identity or binding['disposition'] != 'EXACT'
            or profile['admission'] != {'materializable': True, 'stack_class': 'NonStackable',
                                        'legal_destinations': ['CharacterInventory', 'Ground']}
            or item.get('stack_class') != 'NonStackable'
            or fact(item, 'charges', 'count') != charges
            or fact({'semantics': semantics}, 'charges', 'count') != charges
            or semantics['container'] != {'state': 'NOT_APPLICABLE'}
            or semantics['temporal'] != {'state': 'NOT_APPLICABLE'}):
        return item

    def agrees(accepted, observed):
        if isinstance(accepted, dict):
            return accepted.get('state') == 'UNKNOWN' or isinstance(observed, dict) and all(
                field in observed and agrees(value, observed[field]) for field, value in accepted.items())
        return accepted == observed

    if (not all(agrees(item['semantics'][field], semantics[field]) for field in
                ('physical', 'stack', 'container', 'charges', 'fluid', 'temporal'))
            or any(fact({'semantics': semantics}, 'physical', field) is not True
                   for field in ('movable', 'pickupable'))
            or type(fact({'semantics': semantics}, 'physical', 'weight')) is not int):
        return item
    qualified = copy.deepcopy(item)
    qualified['materializable'] = True
    return qualified


def native_water_vial(item, profile):
    """Use the existing native empty-vial definition, scoped to Dreamer's source claim."""
    if item is None or profile is None:
        return item
    identity = item['identity']
    semantics = profile['semantics']
    if (identity['key'] != 'oteryn:item.tibia.i2874'
            or profile['production_definition'] != {
                'family': 'Item', 'production_key': identity['key'], 'revision_ref': identity['revision']}
            or profile['production_binding']['target'] != identity
            or profile['production_binding']['disposition'] != 'EXACT'
            or profile['admission'] != {'materializable': True, 'stack_class': 'NonStackable',
                                        'legal_destinations': ['CharacterInventory', 'Ground']}
            or semantics['fluid'] != {'state': 'KNOWN', 'value': {'fluid_type': {'state': 'NOT_APPLICABLE'}}}
            or any(semantics[group] != {'state': 'NOT_APPLICABLE'}
                   for group in ('container', 'charges', 'temporal'))):
        return item

    def agrees(accepted, observed):
        if isinstance(accepted, dict):
            return accepted.get('state') == 'UNKNOWN' or isinstance(observed, dict) and all(
                field in observed and agrees(value, observed[field]) for field, value in accepted.items())
        return accepted == observed

    if (not all(agrees(item['semantics'][group], semantics[group])
                for group in ('physical', 'stack', 'container', 'charges', 'fluid', 'temporal'))
            or any(fact({'semantics': semantics}, 'physical', field) is not True
                   for field in ('movable', 'pickupable'))
            or type(fact({'semantics': semantics}, 'physical', 'weight')) is not int):
        return item
    qualified = copy.deepcopy(item)
    qualified['materializable'] = True
    for group in ('fluid', 'container', 'charges', 'temporal'):
        qualified['semantics'][group] = copy.deepcopy(semantics[group])
    return qualified


def claim_items(pilot_key, items):
    water = pilot_key == 'canary:reward-claim/quest/u7_9/dreamers_challenge/chests_wine'
    item = 2874 if water else CHARGED_QUEST_CLAIMS.get(pilot_key)
    if item is None:
        return items
    key = f'oteryn:item.tibia.i{item}'
    root = Path(__file__).resolve().parents[3]
    profiles = json.loads((root / 'content/items/definitions/spell-native-profiles.json').read_bytes())
    matches = [p for p in profiles['records'] if p['production_definition']['production_key'] == key]
    if len(matches) != 1:
        return items
    qualify = native_water_vial if water else native_charge_item
    return {**items, key: qualify(items.get(key), matches[0])}


def source():
    raw = SOURCE.read_bytes()
    if hashlib.sha256(raw).hexdigest() != SOURCE_SHA256:
        raise ValueError('pinned Crystal container source changed')
    value = json.loads(raw)
    if value['revision'] != REVISION or len(value['entries']) != 68:
        raise ValueError('Crystal container inventory changed')
    choices = value['shared_choice_entries']
    if {row['unique_id'] for row in choices} != {6085, 6086, 6087}:
        raise ValueError('shared storage reward choices changed')
    leaves = value['leaf_entries']
    if {row['unique_id'] for row in leaves} != {6013, 6082, 6093, 6112, 6179, 6187,
            6196, 6202, 6203, 6214, 6250, 6272, 6296, 6500}:
        raise ValueError('once-only leaf source inventory changed')
    entries = {row['unique_id']: row for row in value['entries'] + choices + leaves}
    if len(entries) != len(value['entries']) + len(choices) + len(leaves):
        raise ValueError('duplicate Crystal container unique id')
    return value, entries


def known(value):
    return value.get('value') if isinstance(value, dict) and value.get('state') == 'KNOWN' else None


def fact(item, group, field):
    return known((known(item.get('semantics', {}).get(group)) or {}).get(field))


def item_id(ref):
    match = re.fullmatch(r'oteryn:item\.tibia\.i([1-9][0-9]*)', ref.get('key', ''))
    return int(match[1]) if match else None


def full_charge_leaf(item, pilot_key, position, key, raw_count):
    """A pinned leaf is one already-admitted Item at its exact definition charges.

    Missing native timed state means full definition values; this does not
    authorize partial source charges or make a held Item materializable.
    """
    from itemhold_charge_precedence import qualifies
    if qualifies(item, pilot_key, position, key, raw_count):
        return True
    binding = FULL_CHARGE_LEAVES.get(pilot_key)
    if binding is None:
        return False
    unique, source_id, charges = binding
    if (key != f'oteryn:item.tibia.i{source_id}' or raw_count != charges
            or not item or item.get('identity', {}).get('key') != key
            or not item.get('materializable') or item.get('stack_class') != 'NonStackable'
            or fact(item, 'charges', 'count') != raw_count):
        return False
    if unique == 6082:
        _, entries = source()
    else:
        raw = SOURCE.with_name('questpc4_charge_source.json').read_bytes()
        if hashlib.sha256(raw).hexdigest() != CHARGE_SOURCE_SHA256:
            raise ValueError('QUESTPC4_CHARGE_SOURCE_CHANGED')
        evidence = json.loads(raw)
        original, _ = source()
        if (evidence['schema'] != 'OTERYN_QUESTPC4_FULL_CHARGE_SOURCE/v1'
                or any(evidence[field] != original[field] for field in
                       ('revision', 'source_path', 'source_blob_sha1', 'source_sha256'))
                or [row['unique_id'] for row in evidence['entries']] != [6039, 6043, 6051]):
            raise ValueError('QUESTPC4_CHARGE_SOURCE_CHANGED')
        entries = {row['unique_id']: row for row in evidence['entries']}
    setting = entries[unique]['setting']
    return (setting.get('useKV') is True and setting.get('questName', '').lower() == pilot_key.rsplit('/', 1)[1]
            and setting.get('itemPos') == position and setting.get('reward') == [[source_id, charges]])


def derive(definition, items, stack_problem):
    if definition.get('data_holds') or definition['claim'] != {'per': 'character', 'repeat': {'kind': 'once'}}:
        return None
    if not any('container' in p['reward'] or 'written_text' in p['reward']
               or p['reward'].get('key_binding') == 'crystalserver:door-key/3002'
               for p in definition['placements']):
        return None
    evidence, entries = source()
    key_ids = {row['item_id'] for row in evidence['key_item_evidence']['entries']}
    achievements = {row['unique_id']: row['name'] for row in evidence['achievement_entries']}
    texts = {row['unique_id']: row for row in evidence['text_entries']}
    result = []
    for placement in definition['placements']:
        reward = placement['reward']
        if set(reward) - {'container', 'items', 'key_binding', 'written_text'}:
            return None
        ids = [u['unique_id'] for u in placement['source_binding']['legacy_unique_ids'] if u['server'] == 'crystalserver']
        if len(ids) != 1 or ids[0] not in entries:
            return None
        entry = entries[ids[0]]; setting = entry['setting']
        text = texts.get(ids[0])
        written = reward.get('written_text')
        if (written is None) != (text is None):
            return None
        if text is not None:
            # Existing chest source authoring deliberately strips outer LF:
            # quest-authoring/ots_chests.py:278-279. Keep its accepted hash.
            body = text['text'].strip('\n')
            if (not 1 <= len(body.encode()) <= 7996 or len(body) > 4000 or '\0' in body
                    or item_id(written.get('item', {})) != text['item_id']
                    or written.get('text_ref') != {'length': len(body), 'placeholders': [],
                        'sha256': hashlib.sha256(body.encode()).hexdigest()}):
                return None
        positions = setting['itemPos'] if isinstance(setting['itemPos'], list) else [setting['itemPos']]
        if placement['source_binding']['project_position'] not in positions or placement['appearance_tibia_id'] != setting['itemId']:
            return None
        achievement = None
        if ids[0] in achievements:
            grant = placement.get('achievement_grant', {})
            value = grant.get('value', {})
            achievement = value.get('achievement')
            if (achievements[ids[0]] != 'Annihilator' or grant.get('state') != 'KNOWN'
                    or value.get('owner') != 'Achievement' or value.get('request') != 'grant'
                    or achievement != {'family': 'Achievement', 'key': 'oteryn:achievement/annihilator', 'revision': '1'}):
                return None
        elif 'achievement_grant' in placement:
            return None
        if setting.get('useKV'):
            completion = {'kind': 'quest_kv', 'key': setting['questName']}
        else:
            completion = {'kind': 'storage', 'key': setting['storage']['expr']}
        if 'container' not in reward:
            # All four Annihilator choices consume one storage/claim. The three
            # leaf alternatives must be admitted along with the filled present.
            key_leaf = (ids[0] == 6093 and 2969 in key_ids and setting.get('isKey') is True
                        and setting.get('storage') == {'expr': 'Storage.Quest.Key.ID3002'}
                        and reward.get('key_binding') == 'crystalserver:door-key/3002')
            if (ids[0] not in {6085, 6086, 6087} and text is None and not key_leaf
                    or len(reward.get('items', [])) != 1):
                return None
            quantity = reward['items'][0]
            root = items.get(quantity['item']['key'])
            if (quantity['count'] != 1 or setting['reward'] != [[item_id(quantity['item']), 1]]
                    or root is None or root['identity'] != quantity['item']
                    or stack_problem(root, 1) is not None
                    or ('key_binding' in reward and not key_leaf)):
                return None
            if text is not None and (written['item'] != quantity['item']
                    or root.get('stack_class') != 'NonStackable'):
                return None
            leaf = {'unique_id': ids[0], 'line': entry['line'], 'root': copy.deepcopy(quantity['item']),
                    'capacity': None, 'contents': [], 'completion': completion}
            if achievement is not None:
                leaf['achievement'] = achievement
            if text is not None:
                leaf['root_text'] = body
            result.append(leaf)
            continue
        root = items.get(reward['container']['key'])
        capacity = fact(root or {}, 'container', 'capacity')
        if (root is None or root['identity'] != reward['container'] or item_id(reward['container']) != setting['container']
                or stack_problem(root, 1) is not None or type(capacity) is not int or not 1 <= capacity <= 65535):
            return None
        original = setting['reward']
        if ids[0] in ACCEPTED_TWO_STACKS:
            selected = ACCEPTED_TWO_STACKS[ids[0]]
            original = [part for source_id, count in original
                        for part in ([(source_id, 100), (source_id, 100)]
                                     if source_id == selected and count == 200 else [(source_id, count)])]
        if len(original) != len(reward['items']) or len(original) > min(capacity, MAX_CONTENTS):
            return None
        key = setting.get('keyAction', {}).get('expr')
        key_match = re.fullmatch(r'Storage\.Quest\.Key\.ID([0-9]+)', key or '')
        if key is not None and key_match is None:
            return None
        key_number = int(key_match[1]) if key_match else None
        if key_number is not None:
            if not 1 <= key_number <= 65535 or reward.get('key_binding') != f'canary:door-key/{key_match[1]}':
                return None
        elif 'key_binding' in reward:
            return None
        contents = []
        for quantity, (source_id, raw_count) in zip(reward['items'], original):
            item = items.get(quantity['item']['key'])
            count = quantity['count']
            if (item is None or item['identity'] != quantity['item'] or item_id(quantity['item']) != source_id
                    or stack_problem(item, count) is not None):
                return None
            if (count != raw_count
                    and (item.get('stack_class') != 'NonStackable' or count != 1
                         or fact(item, 'charges', 'count') != raw_count)):
                # A non-stackable charged item is one instance. Its absent native
                # timed row means full definition values (item_timed_state.rs).
                return None
            child_capacity = fact(item, 'container', 'capacity')
            if child_capacity is not None and (type(child_capacity) is not int or not 1 <= child_capacity <= 65535 or count != 1):
                return None
            # A qualified source fluid argument is initial state, not a stack.
            # Retain the original holds and closed quest publication scope.
            if known(item.get('semantics', {}).get('fluid')) is not None:
                if definition['provenance']['pilot_key'] not in FLUID_QUEST_CLAIMS:
                    return None
                child = project_fluid_child(quantity, raw_count, item)
                if child is None:
                    return None
            else:
                child = copy.deepcopy(quantity)
            if source_id in key_ids and key_number is not None:
                if count != 1:
                    return None
                child['key_number'] = key_number
            if text is not None and source_id == text['item_id']:
                if count != 1 or item.get('stack_class') != 'NonStackable' or written['item'] != quantity['item']:
                    return None
                child['text'] = body
            contents.append(child)
        if text is not None and sum('text' in child for child in contents) != 1:
            return None
        projected = {'unique_id': ids[0], 'line': entry['line'], 'root': copy.deepcopy(reward['container']),
                     'capacity': capacity, 'contents': contents, 'completion': completion}
        if achievement is not None:
            projected['achievement'] = copy.deepcopy(achievement)
        result.append(projected)
    if not result:
        return None
    return {'schema': 'OTERYN_NATIVE_CONTAINER_REWARD/v1', 'revision': REVISION,
            'source_sha256': SOURCE_SHA256, 'placements': result}
