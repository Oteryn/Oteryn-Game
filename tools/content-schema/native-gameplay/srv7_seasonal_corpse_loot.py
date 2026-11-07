"""Finite seasonal ordinary Channel corpses from pinned Crystal, not Instance admission."""
import json
from pathlib import Path

CAPACITIES = {f'oteryn:item.tibia.i{i}': c for i,c in {8133:15,18114:20,5963:10,5967:10,5968:5,5969:10,5975:20,5989:5,6042:5,9009:44,12325:32}.items()}


def admit(facts, definitions):
    path = definitions / 'spell-native-profiles.json'
    if not path.is_file():
        return
    native = json.loads(path.read_bytes())
    if native['schema'] != 'OTERYN_NATIVE_ITEM_PROFILES/v1':
        raise ValueError('Seasonal corpse requires native Item provider')
    for key, capacity in CAPACITIES.items():
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
            raise ValueError('Seasonal corpse requires exact Item binding: ' + key)
        admission = row['admission']
        if (admission['materializable'] is not True
                or admission['stack_class'] != 'NonStackable'
                or admission['legal_destinations'] != ['Ground']
                or row['semantics'].get('container') != {
                    'state': 'KNOWN', 'value': {'capacity': {'state': 'KNOWN', 'value': capacity}}}):
            continue
        fact['materializable'] = True
        fact['container_capacity'] = capacity
