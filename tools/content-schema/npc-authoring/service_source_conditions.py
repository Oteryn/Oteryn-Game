"""Existing R7 pure source-only previews; no file/network/runtime state access."""

def source_compare(condition, state):
    """Unknown source tracks fail closed, not as fabricated Game initial values."""
    if condition['kind'] != 'SOURCE_TRACK_COMPARE':
        raise ValueError('not a supported source predicate')
    key = condition['source_track']
    if key not in state or type(state[key]) is not int:
        return None
    value, expected = state[key], condition['value']
    if type(expected) is not int:
        raise ValueError('comparison value must be an integer')
    if condition['operator'] == '==':
        return value == expected
    if condition['operator'] == '>=':
        return value >= expected
    raise ValueError('unsupported comparison')

def fare(base, discounts, state):
    """Source-only estimate; no money, quest state or position is written."""
    if type(base) is not int or not 0 <= base <= 1_000_000:
        raise ValueError('invalid fare')
    if len(discounts) > 4:
        raise ValueError('too many discounts')
    applied = 0
    for discount in discounts:
        amount = discount['amount_gold']
        if type(amount) is not int or not 1 <= amount <= 1_000_000:
            raise ValueError('invalid discount amount')
        matched = source_compare(discount['source_condition'], state)
        if matched is None:
            return None
        if matched:
            applied += amount
    return max(0, base - applied)

def transition_preview(effect, state, travel_committed):
    """Proposed safe binding: only preview an outcome after successful travel.

    Canary/Crystal call their action on several travel refusals. We retain that
    defect as source evidence; this proposal cannot be admitted by today's DTO.
    """
    if travel_committed is not True:
        return None
    if source_compare(effect['source_condition'], state) is not True:
        return None
    return {'source_track': effect['source_condition']['source_track'],
            'from': effect['source_condition']['value'], 'to': effect['to']}
