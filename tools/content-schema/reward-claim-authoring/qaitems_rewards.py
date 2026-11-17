"""Explicit quest reward identity corrections; retained donor snapshots stay intact.

Hammer: Crystal 201189f871d23597b82caf88fd611454a2ddefb8,
data-global/startup/tables/chest.lua:1182 gives 3208, not pilot 12510.
For the other three, current Global Tibia wins over OTS. MediaWiki parse API
requests for the quest and item pages returned HTTP 402 on 2026-11-17.
Owner-authorized fallback: lane qa TibiaWiki revs 1148768/1115206/1114329:
Firewalker Boots / The Hidden City of Beregar Quest -> 9018;
Secret Service Quest / Poem Scroll -> 347;
Secret Service Quest / Winning Lottery Ticket -> 5957.
The donor blank-ticket text (quest_reward_common.lua:107-111) belongs to 5956;
it is not text on the selected 5957. That reward is a plain item grant.
"""
import copy

PREFIX = 'canary:reward-claim/quest/'
LOTTERY = PREFIX + 'u8_1/secret_service/lottery_ticket'
CORRECTIONS = {
    PREFIX + 'u7_6/explorer_society/giant_smith_hammer/hammer': (12510, 3208),
    PREFIX + 'u8_4/the_hidden_city_of_beregar/firewalker_boots': (9019, 9018),
    PREFIX + 'u8_1/secret_service/poem_scroll': (5952, 347),
    LOTTERY: (5956, 5957),
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
        if key == LOTTERY and 'written_text' in reward:
            if reward['written_text']['item']['key'] != f'canary:item/{before}':
                raise ValueError('QAITEMS_BLANK_TEXT_CARRIER')
            del reward['written_text']
        if set(reward) != {'items'}:
            raise ValueError('QAITEMS_NOT_PLAIN_REWARD')
        item['key'] = f'canary:item/{after}'
    return selected
