"""Retain the accepted Skull Coin description across the existing source qualifiers."""
import copy


def preserve(existing, observed):
    key = 'oteryn:item.tibia.i32583'
    if observed['production_definition']['production_key'] != key:
        return
    before = existing['semantics']['presentation']
    after = observed['semantics']['presentation']
    expected = lambda text: {'state': 'KNOWN', 'value': {
        'name': {'state': 'KNOWN', 'value': 'skull coin'},
        'description': {'state': 'KNOWN', 'value': text}}}
    if before != expected('The silver is heavily tarnished') or after != expected('The silver is heavily tarnished.'):
        raise ValueError('QUESTPC7_SKULL_COIN_PRESENTATION_DRIFT')
    observed['semantics']['presentation'] = copy.deepcopy(before)
