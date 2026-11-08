"""Retain established mechanic/presentation owners across physical source qualifiers."""
import copy


def preserve(existing, observed):
    if observed['production_definition']['production_key'] == 'oteryn:item.tibia.i3456' and existing['attributes'].get('speed_bonus') is None:
        # MINING-1 owns this unknown mechanic; physical custody cannot assert zero.
        if 'speed_bonus' in existing['attributes']:
            observed['attributes']['speed_bonus'] = None
        else:
            observed['attributes'].pop('speed_bonus', None)
    key = 'oteryn:item.tibia.i32583'
    if observed['production_definition']['production_key'] != key:
        return
    before = existing['semantics']['presentation']
    after = observed['semantics']['presentation']
    expected = lambda text: {'state': 'KNOWN', 'value': {
        'name': {'state': 'KNOWN', 'value': 'skull coin'},
        'description': {'state': 'KNOWN', 'value': text}}}
    if before not in (expected('The silver is heavily tarnished'), expected('The silver is heavily tarnished.')) or after != expected('The silver is heavily tarnished.'):
        raise ValueError('QUESTPC7_SKULL_COIN_PRESENTATION_DRIFT')
    observed['semantics']['presentation'] = copy.deepcopy(before)
