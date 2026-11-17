"""Project the donor's proven literal subtype-one case without guessing clamps.

Other arguments remain held until their constructor clamp/enum mapping is qualified.
The caller retains canonical claim holds, source/position/UID and capacity checks.
"""
import copy


def project_fluid_child(quantity, argument, item):
    if (type(argument) is not int or argument != 1
            or not isinstance(quantity, dict) or set(quantity) != {'item', 'count'}
            or type(quantity['count']) is not int or quantity['count'] != 1
            or item.get('materializable') is not True
            or item.get('stack_class') != 'NonStackable'
            or quantity['item'] != item.get('identity')):
        return None
    semantics = item.get('semantics', {})
    if (semantics.get('fluid') != {'state': 'KNOWN', 'value': {
            'fluid_type': {'state': 'NOT_APPLICABLE'}}}
            or any(semantics.get(group) != {'state': 'NOT_APPLICABLE'}
                   for group in ('charges', 'container', 'temporal'))):
        return None
    return {**copy.deepcopy(quantity), 'fluid_subtype': 1}
