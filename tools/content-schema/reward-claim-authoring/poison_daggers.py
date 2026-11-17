"""Exact target correction for the Poison Daggers chest, not a donor rewrite.

TibiaWiki's Poison Daggers Quest and spoiler, captured 2026-11-06, both specify
two daggers and thirty arrows. Crystal 201189f chest.lua:1087-1096 and Canary
47dfd51f agree in their comments but encode only one dagger. Preserve those
immutable donor facts in source_variant; serialize two physical Item3299s for
the target. Nonstackable quantity2 is never passed to the mint owner.
"""
import copy

PILOT = 'canary:reward-claim/kv/poisondaggers'
POSITION = {'x': 33155, 'y': 31880, 'z': 11}
SOURCE = [[3448, 30], [3299, 1]]
TARGET = [[3448, 30], [3299, 1], [3299, 1]]


def source_reward(unique, position, pairs):
    if unique != 6068:
        return pairs
    if position != POSITION or pairs != SOURCE:
        raise ValueError('POISON_DAGGERS_SOURCE_DRIFT')
    return copy.deepcopy(TARGET)


def correct_canonical(pilot, position, reward):
    if pilot != PILOT:
        return

    def ref(item):
        return {'family': 'Item', 'key': f'oteryn:item.tibia.i{item}',
                'revision': 'definition-r1'}

    expected = {'container': ref(2854),
                'items': [{'item': ref(item), 'count': count} for item, count in SOURCE]}
    if position != POSITION or reward != expected:
        raise ValueError('POISON_DAGGERS_CANONICAL_DRIFT')
    reward['items'].append(copy.deepcopy(reward['items'][1]))
