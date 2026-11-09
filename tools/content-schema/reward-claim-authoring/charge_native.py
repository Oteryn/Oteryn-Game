"""Bounded source-charge recipes, preserving raw 1 as exactly one charge.

This admits reward selectors only; Targuna remains a Crystal-only source and
none of these recipes certify the quest access path. Historical holds remain.
"""
import copy
import hashlib
import json

import container_native as containers

DEFAULTS = {3048: 20, 3054: 200, 3081: 5, 3082: 50, 3083: 150, 3085: 200}
CASES = {
    'oteryn:reward-claim.kv.deeperfibula4': dict(
        uid=6011, appearance=4025, position=(32245, 32492, 10), container=None,
        reward=[(3082, 1)], charged=(0,), completion={'kind': 'quest_kv', 'key': 'deeperfibula4'}),
    'oteryn:reward-claim.quest.u7_2.the_queen_of_the_banshees.reward.stone_skin_amulet': dict(
        uid=6056, appearance=2472, position=(32212, 31896, 15), container=None,
        reward=[(3081, 1)], charged=(0,), completion={'kind': 'storage',
            'key': 'Storage.Quest.U7_2.TheQueenOfTheBanshees.Reward.StoneSkinAmulet'}),
    'oteryn:reward-claim.quest.u15_24.targuna.silver_amulet_chest': dict(
        uid=14093, appearance=28827, position=(31909, 31908, 9), container=None,
        reward=[(3054, 1)], charged=(0,), completion={'kind': 'storage',
            'key': 'Storage.Quest.U15_24.Targuna.SilverAmuletChest'}),
    'oteryn:reward-claim.quest.pre_u6_0.ornamented_shield.rewards.ornamented_shield': dict(
        uid=6062, appearance=4024, position=(32778, 32282, 11), container=2853,
        reward=[(2971, 1), (3509, 1), (3351, 1), (3424, 1), (2821, 1), (3271, 1),
                (3085, 1), (3048, 1)], charged=(6, 7), completion={'kind': 'storage',
            'key': 'Storage.Quest.PreU6_0.OrnamentedShield.Rewards.OrnamentedShield'}),
    'oteryn:reward-claim.kv.ornamentedshield': dict(
        uid=6063, appearance=2480, position=(32769, 32302, 10), container=2859,
        reward=[(2949, 1), (3059, 1), (3083, 1), (3035, 5), (3053, 1)], charged=(2,),
        completion={'kind': 'quest_kv', 'key': 'ornamentedshield'}),
}


def derive(definition, items, stack_problem):
    key = definition.get('identity', {}).get('key')
    spec = CASES.get(key)
    if (spec is None or definition.get('claim') != {'per': 'character', 'repeat': {'kind': 'once'}}
            or len(definition.get('placements', [])) != 1):
        return None
    source = definition.get('provenance', {}).get('pilot_key')
    donor = 'crystalserver' if spec['uid'] == 14093 else 'canary'
    expected_source = donor + ':reward-claim/' + key.removeprefix('oteryn:reward-claim.').replace('.', '/')
    if source != expected_source:
        return None
    placement = definition['placements'][0]
    position = dict(zip(('x', 'y', 'z'), spec['position']))
    binding = placement['source_binding']
    expected_ids = [{'server': server, 'unique_id': spec['uid']}
                    for server in (('crystalserver',) if spec['uid'] == 14093 else ('canary', 'crystalserver'))]
    if (binding != {'project_position': position, 'legacy_unique_ids': expected_ids}
            or placement['appearance_tibia_id'] != spec['appearance']):
        return None
    expected_holds = ([{'category': 'source', 'code': 'SOURCE_CHARGE_MISMATCH',
                       'placement_index': 0, 'reward_field': 'items', 'reward_index': index}
                      for index in spec['charged']] if spec['container'] and spec['uid'] != 6063 else [])
    # Crystal chest.lua:1051, UID6063 explicitly grants [3083, 1]. Its
    # admitted one-charge recipe agrees with Source, not the Item default 150.
    if definition.get('data_holds', []) != expected_holds:
        return None
    reward = placement['reward']
    expected_fields = {'items'} if spec['container'] is None else {'items', 'container'}
    if spec['uid'] == 6062:
        expected_fields.add('key_binding')
        if reward.get('key_binding') != 'canary:door-key/3702':
            return None
    if set(reward) != expected_fields or len(reward['items']) != len(spec['reward']):
        return None
    evidence, entries = containers.source()
    if spec['container']:
        entry = entries[spec['uid']]
        setting = entry['setting']
        if (setting['reward'] != [list(row) for row in spec['reward']]
                or setting['itemPos'] != position or setting['itemId'] != spec['appearance']
                or setting['container'] != spec['container']):
            raise ValueError('pinned partial-charge container changed')
        line = entry['line']
    else:
        raw = containers.CHARGE_EVIDENCE.read_bytes()
        if hashlib.sha256(raw).hexdigest() != containers.CHARGE_EVIDENCE_SHA256:
            raise ValueError('pinned charge source evidence changed')
        matches = [row for row in json.loads(raw)['rewards'] if row['pilot_key'] == source
                   and row['project_position'] == position and row['source_count_argument'] == 1
                   and row['item'] == f"oteryn:item.tibia.i{spec['reward'][0][0]}"
                   and row['source_default_charges'] == DEFAULTS[spec['reward'][0][0]]]
        if len(matches) != 1:
            return None
        witness = next((s for b in matches[0]['source_bindings'] for s in b['sources']
                        if s['source'] == 'crystalserver' and s['uid'] == spec['uid']), None)
        if witness is None:
            return None
        line = witness['source_lines'][0]
    contents = []
    for index, (quantity, (number, count)) in enumerate(zip(reward['items'], spec['reward'])):
        item = items.get(quantity['item']['key'])
        if (item is None or item['identity'] != quantity['item']
                or containers.item_id(quantity['item']) != number or quantity['count'] != count
                or stack_problem(item, count) is not None):
            return None
        child = copy.deepcopy(quantity)
        if index in spec['charged']:
            if (item.get('stack_class') != 'NonStackable'
                    or containers.fact(item, 'charges', 'count') != DEFAULTS[number]):
                return None
            child['charges_remaining'] = 1
        if spec['uid'] == 6062 and number in {e['item_id'] for e in evidence['key_item_evidence']['entries']}:
            child['key_number'] = 3702
        contents.append(child)
    projected = {'unique_id': spec['uid'], 'line': line, 'completion': copy.deepcopy(spec['completion'])}
    if spec['container'] is None:
        projected.update(root=contents[0]['item'], capacity=None, contents=[], root_charges_remaining=1)
    else:
        root = items.get(reward['container']['key'])
        if (root is None or root['identity'] != reward['container']
                or containers.item_id(reward['container']) != spec['container']
                or stack_problem(root, 1) is not None):
            return None
        capacity = containers.fact(root, 'container', 'capacity')
        if type(capacity) is not int or not len(contents) <= capacity <= 65535:
            return None
        projected.update(root=copy.deepcopy(reward['container']), capacity=capacity, contents=contents)
    return {'schema': 'OTERYN_NATIVE_CONTAINER_REWARD/v1', 'revision': containers.REVISION,
            'source_sha256': containers.SOURCE_SHA256, 'placements': [projected],
            'source_difference': 'EXACT_SOURCE_CHARGES_NOT_DEFINITION_DEFAULTS'}
