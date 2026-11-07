"""Current Mutated Pumpkin fixed common loot, isolated from generic W02 semantics.

Current TibiaWiki prose: each supported participant receives five pumpkins and ten
worms. Captured current Wiki SHA256 c5fbb7ddf71779e795806b92038a209770324cb06e6dd610b5335708765ed742.
The pinned Crystal XML resolves names to 3594 and 8177; all rare source entries stay intact.
"""
from copy import deepcopy

TABLE = 'oteryn:loot.creature.the_mutated_pumpkin'
COMMON = {'oteryn:item.tibia.i3594': 5, 'oteryn:item.tibia.i8177': 10}
RARE_ADMISSION = frozenset({'oteryn:item.tibia.i6491', 'oteryn:item.tibia.i6571'})
SOURCE_STACKS = frozenset(COMMON) | {f'oteryn:item.tibia.i{i}' for i in (3599,6569,6574)}


def qualify_table(table):
    if table['identity']['key'] != TABLE:
        return table
    result = deepcopy(table)
    if result['algorithm'] != 'IndependentBernoulliPpm':
        raise ValueError('Pumpkin source common rewards require the admitted independent table')
    for key, count in COMMON.items():
        entries = [entry for entry in result['entries'] if entry['item']['key'] == key]
        if (len(entries) != 1 or entries[0]['item']['family'] != 'Item'
                or entries[0]['probability_ppm'] != 1000000 or entries[0].get('children')):
            raise ValueError('Pumpkin source requires one exact guaranteed common entry: ' + key)
        entries[0]['min_count'] = count
        entries[0]['max_count'] = count
    return result


def admit_stacks(facts, native):
    """Only exact active native Item bindings can establish common-reward stack semantics."""
    for key in SOURCE_STACKS | RARE_ADMISSION:
        fact = facts.get(key)
        if fact is None:
            continue
        rows = [row for row in native['records']
                if row['production_definition']['production_key'] == key]
        if len(rows) != 1:
            continue
        row = rows[0]
        ref = row['production_definition']
        expected = {'family': 'Item', 'key': key, 'revision': ref['revision_ref']}
        if (ref['family'] != 'Item' or fact['item'] != expected
                or row['production_binding']['disposition'] != 'EXACT'
                or row['production_binding']['target'] != expected):
            raise ValueError('Pumpkin common stack admission requires exact native binding')
        stack = row['semantics'].get('stack', {})
        admission = row['admission']
        physical = row['semantics'].get('physical', {})
        inventory_source = (admission['materializable'] is True
                            and 'CharacterInventory' in admission['legal_destinations']
                            and physical.get('state') == 'KNOWN'
                            and physical['value']['pickupable'] == {'state': 'KNOWN', 'value': True})
        if (key in RARE_ADMISSION and inventory_source
                and admission['stack_class'] == 'NonStackable'
                and row['semantics'].get('container') == {'state': 'NOT_APPLICABLE'}):
            fact['materializable'] = True
        if (key in SOURCE_STACKS and inventory_source
                and admission['stack_class'] == 'StackCapable'
                and stack.get('state') == 'KNOWN'
                and stack['value']['stackable'] == {'state': 'KNOWN', 'value': True}
                and stack['value']['stack_max'] == {'state': 'KNOWN', 'value': 100}):
            fact['stackable'] = True
