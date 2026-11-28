"""Validate the executable Crystal 201189f chest.lua:1089-1096 recipe.

The comment and TibiaWiki count two daggers. The numeric reward grants
one dagger and thirty arrows.
"""

PILOT = 'canary:reward-claim/kv/poisondaggers'
POSITION = {'x': 33155, 'y': 31880, 'z': 11}
SOURCE = [[3448, 30], [3299, 1]]


def source_reward(unique, position, pairs):
    if unique != 6068:
        return pairs
    if position != POSITION or pairs != SOURCE:
        raise ValueError('POISON_DAGGERS_SOURCE_DRIFT')
    return pairs


def verify_canonical(pilot, position, reward):
    if pilot != PILOT:
        return

    def ref(item):
        return {'family': 'Item', 'key': f'oteryn:item.tibia.i{item}',
                'revision': 'definition-r1'}

    expected = {'container': ref(2854),
                'items': [{'item': ref(item), 'count': count} for item, count in SOURCE]}
    if position != POSITION or reward != expected:
        raise ValueError('POISON_DAGGERS_CANONICAL_DRIFT')
