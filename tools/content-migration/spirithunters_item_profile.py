"""Admit Crystal's reusable Magical Measurement Device for Spectulus and grave USE."""
import qengine_reward_item_profiles as items
import quest_reward_item_semantics as semantics

KEY = 'oteryn:item.tibia.i4049'


def capture(root):
    record, = items.capture(root, {4049})['records']
    if (record['production_definition']['production_key'] != KEY
            or record['admission'] != {
                'materializable': True, 'stack_class': 'NonStackable',
                'legal_destinations': ['CharacterInventory', 'Ground'],
            }):
        raise ValueError('Spirithunters device custody drift')
    return record


def apply_admissions(definitions, root):
    definition, = (row for row in definitions if row['identity']['key'] == KEY)
    record = capture(root)
    for group in ('physical', 'stack'):
        semantics.put_group(definition, group, record['semantics'][group]['value'])
    definition['stack_class'] = record['admission']['stack_class']
    definition['materializable'] = True


def admit(root, generated, profile_path, manifest_path, order, canonical):
    items.admit_records(root, generated, profile_path, manifest_path, order, canonical, [capture(root)])
