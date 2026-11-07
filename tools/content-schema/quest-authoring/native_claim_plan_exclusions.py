"""Identify explicitly bound claims; this is not a quest qualification verdict."""
import quest_claim_progress


def declared_native_claim(root, quest, transition):
    source = transition.get('source') or {}
    if source.get('owner') != 'chest':
        return None
    rows = [row for row in quest_claim_progress.load(root)
            if row['transition'] == transition['key']]
    if len(rows) != 1:
        raise ValueError('unregistered native chest transition: ' + transition['key'])
    row = rows[0]
    expected = [{'track': row['track'],
                 'from': {'op': 'EQ', 'value': row['from']} if 'from' in row else {'op': 'ANY'},
                 'from_exact': True, 'effect': {'kind': 'SET', 'value': row['value']}}]
    if (row['quest'] != quest or transition.get('quest') != quest
            or transition.get('requested_by') is not None
            or transition.get('completes') is not False
            or transition.get('effects') != expected
            or source.get('script') != row['source']['path']):
        raise ValueError('native chest binding differs from declared claim: ' + transition['key'])
    return {'quest': quest, 'transition': transition['key'], 'claim': row['claim'],
            'basis': 'EXISTING_NATIVE_CLAIM_BINDING_NOT_A_MISSING_TRIGGER'}


def claim_only_owner(root, quest):
    """A native-only claim does not constitute donor mission coverage."""
    return (quest['completion'] == 'NOT_LOWERED_NO_MISSIONS'
            and bool(quest['transitions'])
            and all((row.get('source') or {}).get('owner') == 'chest'
                    and declared_native_claim(root, quest['quest'], row) is not None
                    for row in quest['transitions'])
            and all(row.get('bounds_basis') == 'native_claim_source_binding'
                    for row in quest['tracks']))
