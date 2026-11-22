"""Bind the retained Doomforge chest row to the eleven Crystal reward choices."""
from hashlib import sha256
from pathlib import Path
import json

from engclaim_native import canonical

ROOT = Path(__file__).resolve().parents[3]
SHARD = 'content/interactions/reward_claims/nrwicked-native.json'
CALLBACKS = 'content/quests/native-reward-callbacks/nrwicked.json'
SOURCE = 'docs/reference/content-prep/nrwicked/sources.md'
QUEST = 'oteryn:quest.no_rest_for_the_wicked_quest'
CLAIM = 'oteryn:reward-claim.nrwicked-forgemaster'
REVISION = 'nrwicked-r1'
TRACK = 'oteryn:quest-progress/crystalserver/quest/u14_10/no_rest_for_the_wicked/'
ACCESS = TRACK + 'forge_master/access_door'
DOOR = TRACK + 'forgemaster_door/questline'
CLAIMED = TRACK + 'forgemaster_door/reward'
# Exact Crystal reward script IDs and XML names, in ascending source UID order.
REWARDS = (
    (49520, 'inferniarch bow'), (49522, 'inferniarch arbalest'),
    (49523, 'inferniarch battleaxe'), (49524, 'inferniarch greataxe'),
    (49525, 'inferniarch flail'), (49526, 'inferniarch warhammer'),
    (49527, 'inferniarch blade'), (49530, 'inferniarch slayer'),
    (49528, 'inferniarch wand'), (49529, 'inferniarch rod'),
    (50250, 'inferniarch claws'),
)


def check(track, value):
    return {'track': track, 'minimum': value, 'maximum': value}


def records():
    placements, callbacks = [], []
    base = {'quest': QUEST, 'npc': '', 'keywords': [], 'topic': 0, 'next_topic': 0,
            'take': [], 'present': [], 'outfit': None, 'achievement': None, 'sources': [SOURCE]}
    for index, (item_id, name) in enumerate(REWARDS):
        callback = f'oteryn:quest-reward/nrwicked-forgemaster-{57237 + index}'
        item = f'oteryn:item.tibia.i{item_id}'
        placements.append({
            'appearance_tibia_id': 2472,
            'source_binding': {
                'project_position': {'x': 34087 + 2 * index, 'y': 32346, 'z': 14},
                'native_entry_ordinal': 3 if index == 10 else 2,
            },
            'reward': {'items': [{'item': {'family': 'Item', 'key': item,
                                            'revision': 'definition-r1'}, 'count': 1}]},
            'reward_callback': callback,
        })
        callbacks.append({
            **base, 'key': callback, 'requires': [check(ACCESS, 1), check(DOOR, 3)],
            'storage': [{**check(CLAIMED, 0), 'set': 1}],
            'give': {'item': item, 'count': 1, 'contents': []},
            'reply': [f'You have found {name}.'],
        })
    terms = {'identity': {'key': CLAIM, 'revision': REVISION},
             'claim': {'per': 'character', 'repeat': {'kind': 'once'}},
             'placements': placements}
    digest = sha256(canonical(terms).rstrip(b'\n')).hexdigest()
    for callback in callbacks:
        callback['world_claim'] = {'key': CLAIM, 'revision': REVISION, 'authority_sha256': digest}
    callbacks.append({
        **base, 'key': 'oteryn:quest-reward/nrwicked-forgemaster-key',
        'npc': 'oteryn:npc.the_forgemaster', 'keywords': ['key'],
        'storage': [{**check(ACCESS, 0), 'set': 1}, {**check(DOOR, 2), 'set': 3}],
        'give': None,
        'reply': ["You have proven yourself worthy. The door to the Forgemaster's chamber is now accessible."],
        'failure_reply': ['You are not ready yet. Proove yourself worthy first.'],
    })
    definition = {
        **terms, 'definition_profile': 'native_authored_v1', 'readiness': 'ready',
        'provenance': {'pilot_key': CLAIM, 'pilot_revision': REVISION, 'path': SOURCE,
                       'sha256': sha256((ROOT / SOURCE).read_bytes()).hexdigest()},
    }
    return [{'definition': definition}], {'schema': 1, 'callbacks': callbacks}


def extend_outputs(outputs, compact):
    claims, callbacks = records()
    outputs[SHARD] = compact({
        'family': 'RewardClaim', 'schema': 'OTERYN_REWARD_CLAIM_SHARD/v1',
        'shard': {'count': 1, 'start': 0, 'end': 0, 'index': 6}, 'records': claims,
    })
    outputs[CALLBACKS] = canonical(callbacks).decode()
    index_path = 'content/interactions/reward_claims/index.json'
    index = json.loads(outputs[index_path])
    index['shards'].append(SHARD)
    index['native_authored_records'] += 1
    index['record_count'] += 1
    index['readiness']['ready'] += 1
    outputs[index_path] = compact(index)
    return claims
