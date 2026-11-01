"""Lower complete canonical container variants against the pinned Global handler.

Source dispositions remain intact. `native_contents` is a separate, reproducible
admission of the exact root and every child; a missing child never becomes an
empty or partial reward. Source conflicts and other reward formats stay held.
"""
import copy
import hashlib
import json
import re
from pathlib import Path

SOURCE = Path(__file__).with_name('container_native_source.json')
SOURCE_SHA256 = 'b073d546930a60adf511677f932e6e0d37df3a230313dc85c4b9253742325757'
REVISION = '201189f871d23597b82caf88fd611454a2ddefb8'
MAX_CONTENTS = 20  # Existing BAGS-0 direct-container admission, not a donor capacity.
# Existing selected owner rulings, reaffirmed by PR2770 ANSWER 6542716086/Q4.
# Their source total remains 200; this deliberately preserves the accepted
# native two-stack recipe over Container:addItem's donor clamp to one stack.
ACCEPTED_TWO_STACKS = {6218: 3031, 6226: 3450}


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
    """UID6082 is one already-admitted Item at its exact definition charges.

    Missing native timed state means full definition values; this does not
    authorize partial source charges or make a held Item materializable.
    """
    if (pilot_key != 'canary:reward-claim/kv/madmageroom3'
            or key != 'oteryn:item.tibia.i3081' or raw_count != 5
            or not item or item.get('identity', {}).get('key') != key
            or not item.get('materializable') or item.get('stack_class') != 'NonStackable'
            or fact(item, 'charges', 'count') != raw_count):
        return False
    _, entries = source()
    setting = entries[6082]['setting']
    return (setting.get('useKV') is True and setting.get('questName') == 'madmageroom3'
            and setting.get('itemPos') == position and setting.get('reward') == [[3081, 5]])


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
            # A source fluid argument is a subtype, never a stack count. The
            # immutable source compiler records those separately; do not erase it.
            if known(item.get('semantics', {}).get('fluid')) is not None:
                return None
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
