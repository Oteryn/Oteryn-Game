"""Reconcile overlapping native producers without duplicate identities or lost facts."""
import copy
from grizzly_shop_profiles import accepted_first, known_conflict


def reconcile(existing, proposed, error):
    """Keep canonical display text; require every proposed mechanical fact to agree."""
    semantics = accepted_first(existing['semantics'], proposed['semantics'])
    mechanics = lambda value: {k: v for k, v in value.items() if k != 'presentation'}
    if (existing['production_definition'] != proposed['production_definition']
            or existing['production_binding'] != proposed['production_binding']
            or existing['admission'] != proposed['admission']
            or known_conflict(mechanics(proposed['semantics']), mechanics(semantics))):
        raise ValueError(error + ':' + proposed['production_definition']['production_key'])
    attributes = copy.deepcopy(existing['attributes'])
    for key, value in proposed['attributes'].items():
        if attributes.get(key) is not None and value is not None and attributes[key] != value:
            raise ValueError(error + ':attribute:' + key)
        if attributes.get(key) is None:
            attributes[key] = copy.deepcopy(value)
    merged = copy.deepcopy(existing)
    merged['semantics'] = semantics
    merged['attributes'] = attributes
    return merged
