"""Limit a physical custody contribution to its three qualified semantic groups."""
import copy


def observation(existing, proposed, join):
    scoped = copy.deepcopy(proposed)
    if 'attributes' in scoped:
        attributes = copy.deepcopy(existing.get('attributes', {}))
        physical = {'blocks_movement', 'blocks_projectile', 'has_height', 'immovable_block_solid'}
        for key, value in proposed['attributes'].items():
            if key in physical or (key not in attributes and (value is None or value == [])):
                attributes[key] = copy.deepcopy(value)
        scoped['attributes'] = attributes
    semantics = copy.deepcopy(join(existing['semantics'], proposed['semantics']))
    for group in ('physical', 'stack', 'container'):
        if group in proposed['semantics']:
            semantics[group] = copy.deepcopy(proposed['semantics'][group])
    scoped['semantics'] = semantics
    return scoped
