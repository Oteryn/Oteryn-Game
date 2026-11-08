"""Select one proved direct leaf without rewriting the source bag or once claim.

The caller stores this projection as ``definition['native_contents']``. Source
qualification is delegated to container_native; held source rows stay held.
Paths are zero-based direct-child ordinals, matching the native container plan.
"""
import copy

import container_native


def derive(definition, items, stack_problem, *, source_unique_id, path):
    if (type(source_unique_id) is not int or not 1 <= source_unique_id <= 65535
            or not isinstance(path, (list, tuple)) or len(path) != 1
            or type(path[0]) is not int or not 0 <= path[0] < container_native.MAX_CONTENTS):
        raise ValueError('selected leaf requires an exact source UID and one bounded ordinal')
    original = container_native.derive(definition, items, stack_problem)
    if original is None:
        raise ValueError('selected leaf requires a complete qualified source container')
    projection = copy.deepcopy(original)
    matches = [r for r in projection['placements'] if r['unique_id'] == source_unique_id]
    if len(matches) != 1:
        raise ValueError('selected leaf source UID is missing or ambiguous')
    recipe = matches[0]
    if recipe.get('capacity') is None or path[0] >= len(recipe['contents']):
        raise ValueError('selected leaf is not a direct child of the source container')
    child = recipe['contents'][path[0]]
    recipe['selected_leaf'] = {
        'source_unique_id': source_unique_id,
        'path': list(path),
        'item': copy.deepcopy(child['item']),
        'count': child['count'],
    }
    return projection
