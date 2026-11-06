"""Pinned native carrying facts for Pemaret's fish and trophy.

The original XML-only canonical class of 901 predates client flag admission.
The served client marks it cumulative, the engine reads that exact flag, and
its canonical stack semantics are UNKNOWN. Refine only that exact old record
for native custody; never generalize the exception to another Item.
"""
import copy
import hashlib
import json
import sys

from fathers_burden_item_profiles import CLIENT

KEYS = {'oteryn:item.tibia.i901', 'oteryn:item.tibia.i902'}


def definitions(root, definitions, source_items):
    raw = (root / CLIENT).read_bytes()
    if hashlib.sha256(raw).hexdigest() != CLIENT.rsplit('-', 1)[1][:-4]:
        raise ValueError('MARLIN_CLIENT_PIN')
    sys.path.insert(0, str(root / 'tools/content-schema/item-authoring'))
    from engine_items import load_appearance_objects
    appearances = load_appearance_objects(raw)
    result = dict(definitions)
    for item, name, weight, stackable in ((901, 'marlin', 1800, True),
                                         (902, 'marlin trophy', 500, False)):
        key = f'oteryn:item.tibia.i{item}'
        definition = copy.deepcopy(definitions[key])
        appearance = appearances.get(item, {})
        flags = appearance.get('flags', {})
        sources = [source for source in source_items if source.get('id') == str(item)]
        if (appearance.get('name') != name or flags.get('flags.take') is not True
                or flags.get('flags.cumulative', False) != stackable
                or any(flags.get(f'flags.{flag}', False) for flag in ('unmove', 'container'))
                or len(sources) != 1 or sources[0].get('name') != name
                or [attr.get('value') for attr in sources[0].findall("attribute[@key='weight']")] != [str(weight)]):
            raise ValueError('MARLIN_SOURCE_FACTS')
        semantics = definition['semantics']
        if (semantics['presentation']['value']['name'] != {'state': 'KNOWN', 'value': name}
                or semantics['physical']['value']['weight'] != {'state': 'KNOWN', 'value': weight}
                or semantics['container']['state'] != 'UNKNOWN'):
            raise ValueError('MARLIN_CANONICAL_FACTS')
        if item == 901:
            digest = hashlib.sha256(json.dumps(definition, sort_keys=True,
                                               separators=(',', ':')).encode()).hexdigest()
            if digest != '8c702d758e146c5a10cc051ea8dc1c1b36bec4991691ef26a6911ebaf7c3d033':
                raise ValueError('MARLIN_CANONICAL_STACK_SUPERSESSION')
        elif definition['stack_class'] != 'NonStackable' or semantics['stack']['value']['stackable'] != {'state': 'KNOWN', 'value': False}:
            raise ValueError('MARLIN_CANONICAL_FACTS')
        for field in ('movable', 'pickupable'):
            if semantics['physical']['value'][field] not in ({'state': 'UNKNOWN'}, {'state': 'KNOWN', 'value': True}):
                raise ValueError('MARLIN_CANONICAL_FACTS')
            semantics['physical']['value'][field] = {'state': 'KNOWN', 'value': True}
        semantics['container'] = {'state': 'NOT_APPLICABLE'}
        semantics['stack'] = {'state': 'KNOWN', 'value': {
            'stackable': {'state': 'KNOWN', 'value': stackable},
            'stack_max': {'state': 'KNOWN', 'value': 100} if stackable else {'state': 'UNKNOWN'}}}
        definition.update(materializable=True, stack_class='StackCapable' if stackable else 'NonStackable')
        result[key] = definition
    return result
