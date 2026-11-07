"""Native custody for the six missing QREADY2 reward Items; no new semantics.

Reuse quest_callback_profiles' canonical admission and exact Crystal binding
checks, as used by the older QUESTPC catalogue qualification. All other reward
Items in this lane already belong to the active provider.
"""
import json

IDS = (21982, 2822, 2823, 2848, 3310, 3587)
PACKET = 'tools/content-schema/quest-authoring/samples/completion-matrix/qready2-cases.json'
PURPLE = 'tools/content-schema/quest-authoring/samples/completion-matrix/qready2-purple-cases.json'


def keys(root):
    claims = {case['claim_key'] for path in (PACKET, PURPLE) for case in json.loads((root / path).read_bytes())['cases']}
    rewards = set()
    index = json.loads((root / 'content/interactions/reward_claims/index.json').read_bytes())
    for path in index['shards']:
        for row in json.loads((root / path).read_bytes())['records']:
            definition = row['definition']
            if definition['identity']['key'] in claims:
                if definition.get('readiness') != 'ready':
                    raise ValueError('QREADY2_REWARD_NOT_READY')
                rewards.update(item['item']['key'] for placement in definition['placements']
                               for item in placement['reward']['items'])
    selected = {f'oteryn:item.tibia.i{number}' for number in IDS}
    if not selected <= rewards:
        raise ValueError('QREADY2_NATIVE_ITEM_OUTSIDE_OWNED_REWARDS')
    return selected
