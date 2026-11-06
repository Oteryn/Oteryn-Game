"""Native carrying facts for Tereban's eight fixed hand-in materials.

The served client proves portability and nonstacking; the pinned Canary XML
and existing canonical records agree on every name and weight. This overlay
grants carrying admission only, without authorizing the crystal's teleport use.
"""
import copy
import hashlib
import sys

CLIENT = 'content/assets/files/appearances-2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.dat'
ITEMS = {
    11545: ('exquisite silk', 10), 11546: ('spectral cloth', 500),
    11547: ('exquisite wood', 500), 11548: ('strong sinew', 35),
    11549: ('old iron', 222), 11550: ('flexible dragon scale', 80),
    11551: ('mystic root', 60), 11552: ('magic crystal', 140),
}
KEYS = {f'oteryn:item.tibia.i{item}' for item in ITEMS} | {'oteryn:item.tibia.i11701'}


def definitions(root, definitions, source_items):
    """Return independent qualified copies; reject any source or identity drift."""
    raw = (root / CLIENT).read_bytes()
    if hashlib.sha256(raw).hexdigest() != CLIENT.rsplit('-', 1)[1][:-4]:
        raise ValueError('FATHERS_BURDEN_CLIENT_PIN')
    sys.path.insert(0, str(root / 'tools/content-schema/item-authoring'))
    from engine_items import load_appearance_objects
    appearances = load_appearance_objects(raw)
    result = dict(definitions)
    for item, (name, weight) in ITEMS.items():
        key = f'oteryn:item.tibia.i{item}'
        definition = copy.deepcopy(definitions[key])
        appearance = appearances.get(item, {})
        flags = appearance.get('flags', {})
        sources = [source for source in source_items if source.get('id') == str(item)]
        if (appearance.get('name') != name or flags.get('flags.take') is not True
                or any(flags.get(f'flags.{flag}', False) for flag in ('unmove', 'container', 'cumulative'))
                or len(sources) != 1 or sources[0].get('name') != name
                or [attr.get('value') for attr in sources[0].findall("attribute[@key='weight']")] != [str(weight)]):
            raise ValueError('FATHERS_BURDEN_SOURCE_FACTS')
        semantics = definition['semantics']
        if (semantics['presentation']['value']['name'] != {'state': 'KNOWN', 'value': name}
                or semantics['physical']['value'] != {
                    'weight': {'state': 'KNOWN', 'value': weight},
                    'movable': {'state': 'KNOWN', 'value': True},
                    'pickupable': {'state': 'KNOWN', 'value': True}}
                or semantics['stack']['value']['stackable'] != {'state': 'KNOWN', 'value': False}
                or definition['stack_class'] not in ('Unknown', 'NonStackable')
                or semantics['container']['state'] not in ('UNKNOWN', 'NOT_APPLICABLE')):
            raise ValueError('FATHERS_BURDEN_CANONICAL_FACTS')
        definition.update(materializable=True, stack_class='NonStackable')
        semantics['container'] = {'state': 'NOT_APPLICABLE'}
        result[key] = definition
    return result
