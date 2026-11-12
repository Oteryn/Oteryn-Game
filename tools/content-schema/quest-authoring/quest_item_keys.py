"""Add catalogue references to chosen quest recipes; never infer native admission.

Targets remain strings for existing consumers. target_items is their additive,
ordered name/reference projection. Unknown objects stay unresolved unless explicitly
classified below; no aliases, pluralization, trimming or numeric-ID inference.
"""
from collections import Counter
import json

ITEM_STEPS = frozenset(('item', 'collect', 'use', 'use-with-item', 'deliver', 'delivery'))
NON_ITEMS = frozenset(('dead trees',))


def item_definitions(root):
    return [row['definition']
            for path in sorted((root / 'content/items/definitions').glob('items-*.json'))
            for row in json.loads(path.read_text(encoding='utf-8'))['records']]


def catalogue(definitions):
    index = {}
    for definition in definitions:
        presentation = definition.get('semantics', {}).get('presentation', {})
        name = presentation.get('value', {}).get('name', {})
        if presentation.get('state') == 'KNOWN' and name.get('state') == 'KNOWN':
            index.setdefault(name['value'].casefold(), []).append(dict(definition['identity']))
    return index


def resolve(name, index):
    candidates = index.get(name.casefold(), [])
    if len(candidates) == 1:
        return {'name': name, 'item': dict(candidates[0])}
    status = ('ambiguous' if candidates else
              'not_an_item' if name.casefold() in NON_ITEMS else 'unresolved')
    return {'name': name, 'item_resolution': status}


def enrich(value, index):
    counts = Counter({status: 0 for status in ('resolved', 'ambiguous', 'unresolved', 'not_an_item')})
    names = {}

    def entry(name):
        result = resolve(name, index)
        status = result.get('item_resolution', 'resolved')
        counts[status] += 1
        if status != 'resolved':
            row = names.setdefault(name, {'name': name, 'item_resolution': status,
                                         'candidate_keys': sorted(c['key'] for c in index.get(name.casefold(), [])),
                                         'occurrences': 0})
            row['occurrences'] += 1
        return result

    def walk(node):
        if isinstance(node, list):
            for child in node:
                walk(child)
        elif isinstance(node, dict):
            if node.get('kind') in ITEM_STEPS and isinstance(node.get('targets'), list):
                node['target_items'] = [entry(name) for name in node['targets']]
            if isinstance(node.get('reward_intents'), list):
                for reward in node['reward_intents']:
                    if reward.get('kind') == 'item':
                        reward.pop('item', None)
                        reward.pop('item_resolution', None)
                        reward.update(entry(reward['name']))
            for key, child in list(node.items()):
                if key != 'target_items':
                    walk(child)

    walk(value)
    return {'schema': 'OTERYN_QUEST_ITEM_KEY_RESOLUTION/v1',
            'counts': dict(counts), 'names': [names[name] for name in sorted(names)]}


def validate_references(value, definitions):
    identities = {d['identity']['key']: d['identity'] for d in definitions}

    def walk(node):
        if isinstance(node, list):
            for child in node:
                walk(child)
        elif isinstance(node, dict):
            item = node.get('item')
            if (isinstance(item, dict) and item.get('family') == 'Item'
                    and str(item.get('key', '')).startswith('oteryn:')):
                if item != identities.get(item.get('key')):
                    raise ValueError(f'Quest Item reference missing or stale: {item}')
            for child in node.values():
                walk(child)

    walk(value)


def extend_completion_schema(here, schema):
    """Extend the final tree schema while retaining the pinned SOURCE vocabulary."""
    import copy
    schema = copy.deepcopy(schema)
    extension = json.loads((here / 'quest_item_keys.schema.json').read_text(encoding='utf-8'))
    properties = schema['properties']['recipe']['properties']
    properties['stages']['items']['properties']['target_items'] = extension['target_items']
    properties['reward_intents']['items']['properties'].update(
        {name: extension[name] for name in ('item', 'item_resolution')})
    return schema


def without_enrichment(recipe):
    """Remove only the derived fields at their defined recipe positions."""
    import copy
    result = copy.deepcopy(recipe)
    for stage in result['stages']:
        stage.pop('target_items', None)
    for reward in result['reward_intents']:
        reward.pop('item', None)
        reward.pop('item_resolution', None)
    return result
