"""Compose the exact source ladder fact with main's stationary projection policy."""
import copy


def compose(existing, source):
    """Return a composition only when these are the two exact compatible facts."""
    if source['production_definition']['production_key'] != 'oteryn:item.tibia.i1948':
        return None
    expected = copy.deepcopy(source)
    if (expected['admission'] != {'materializable': True,
            'stack_class': 'NonStackable', 'legal_destinations': ['Ground']}
            or expected['semantics']['classification']['value']['item_type']
            != {'state': 'KNOWN', 'value': 'LADDER'}):
        return None
    expected['admission']['materializable'] = False
    expected['semantics']['classification']['value']['item_type'] = {'state': 'NOT_APPLICABLE'}
    if existing != expected:
        return None
    result = copy.deepcopy(source)
    result['admission']['materializable'] = False
    return result


def stationary_tool(existing, source):
    """Preserve the exact immovable 1791 floor operand without mint authority."""
    if source['production_definition']['production_key'] != 'oteryn:item.tibia.i1791':
        return None
    expected = copy.deepcopy(source)
    if expected['admission'] != {'materializable': True,
            'stack_class': 'NonStackable', 'legal_destinations': ['Ground']}:
        return None
    expected['admission']['materializable'] = False
    return expected if existing == expected else None
