"""Explicit quest reward identity corrections; retained donor snapshots stay intact.

Hammer: Crystal 201189f871d23597b82caf88fd611454a2ddefb8,
data-global/startup/tables/chest.lua:1182 gives 3208, not pilot 12510.
Wiki API requests returned HTTP 402 on 2026-11-17. Owner review selects:
- Firewalker boots: chest.lua:2720 gives 9019. data/items/items.xml:24049-24054
  names this inventory form "firewalker boots" and transforms it to equipped 9018.
- Lottery: chest.lua:2533 gives 5956. data/items/items.xml:16182-16187 calls it
  "lottery ticket" ("Unfortunately it seems that you drew a blank").
  quest_reward_common.lua:107-111 supplies literal Secret Service quest text.
  Keep that source reward intact; container_native derives the existing text grant.
- Poem: owner-authorized lane qa TibiaWiki rev 1115206 still selects 347.
"""
import copy

PREFIX = 'canary:reward-claim/quest/'
CORRECTIONS = {
    PREFIX + 'u7_6/explorer_society/giant_smith_hammer/hammer': (12510, 3208),
    PREFIX + 'u8_1/secret_service/poem_scroll': (5952, 347),
}


def selected_claims(claims):
    selected = copy.deepcopy(claims)
    for claim in selected:
        key = claim['identity']['key']
        if key not in CORRECTIONS:
            continue
        before, after = CORRECTIONS[key]
        if claim['claim'] != {'per': 'character', 'repeat': {'kind': 'once'}} or len(claim['placements']) != 1:
            raise ValueError('QAITEMS_CLAIM_SHAPE')
        reward = claim['placements'][0]['reward']
        if len(reward['items']) != 1 or reward['items'][0]['count'] != 1:
            raise ValueError('QAITEMS_REWARD_SHAPE')
        item = reward['items'][0]['item']
        if item['key'] not in (f'canary:item/{before}', f'canary:item/{after}'):
            raise ValueError('QAITEMS_SOURCE_ID_DRIFT')
        if set(reward) != {'items'}:
            raise ValueError('QAITEMS_NOT_PLAIN_REWARD')
        item['key'] = f'canary:item/{after}'
    return selected
