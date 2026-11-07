"""Preserve canonical authoring when the source importer retains a candidate alias."""


def canonical_authoring(prior, record):
    binding = prior.get('production_binding') or {}
    key = record['production_definition']['production_key']
    target = binding.get('target')
    return (binding == record.get('production_binding')
            and binding.get('disposition') == 'EXACT'
            and prior['production_definition'] == record['production_definition']
            and prior['authoring'].get('item') == target
            and target == {'family': 'Item', 'key': key,
                           'revision': record['production_definition']['revision_ref']}
            and record['authoring'] == {'item': {'family': 'Item',
                'key': 'candidate:item/' + key.rsplit('i', 1)[-1],
                'revision': 'spell-p2-r21'}})
