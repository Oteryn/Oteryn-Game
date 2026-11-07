"""Closed S06 producer operands, including real Demon Skeleton/Stampor loot; generic engine unchanged."""
import json
PROMOTED = frozenset({3415,3062,9647,12312,12313,12314,10304,11472,3056,3264,3274,3371,3410,3413,7368,7421,7452,8896,9056,9692})
STACKED = frozenset({3062,3078,3030,3027,268,3287,266,9647,12314,236,237,12312,12313,3035,10304,11472,239,3031,3492,3577,3582,3606,5878,5890,5896,5897,5902,6499,7368,7439,7643,9057,9692})


def admit(facts, definitions):
    path = definitions / 'spell-native-profiles.json'
    if not path.is_file():
        return
    native = json.loads(path.read_bytes())
    if native['schema'] != 'OTERYN_NATIVE_ITEM_PROFILES/v1':
        raise ValueError('Acquisition loot requires active native Item provider')
    for identifier in PROMOTED | STACKED:
        key = f'oteryn:item.tibia.i{identifier}'
        fact = facts.get(key)
        rows = [r for r in native['records'] if r['production_definition']['production_key'] == key]
        if fact is None or len(rows) != 1:
            continue
        row = rows[0]
        ref = row['production_definition']
        expected = {'family':'Item','key':key,'revision':ref['revision_ref']}
        if (ref['family'] != 'Item' or fact['item'] != expected
                or row['production_binding']['disposition'] != 'EXACT'
                or row['production_binding']['target'] != expected):
            raise ValueError('Acquisition loot requires exact active native binding: ' + key)
        admission = row['admission']
        physical = row['semantics'].get('physical',{})
        if (admission['materializable'] is not True
                or 'CharacterInventory' not in admission['legal_destinations']
                or physical.get('state') != 'KNOWN'
                or physical['value']['pickupable'] != {'state':'KNOWN','value':True}):
            continue
        if identifier in PROMOTED:
            fact['materializable'] = True
        stack = row['semantics'].get('stack',{})
        if (identifier in STACKED and admission['stack_class'] == 'StackCapable'
                and stack == {'state':'KNOWN','value':{'stackable':{'state':'KNOWN','value':True},'stack_max':{'state':'KNOWN','value':100}}}):
            fact['stackable'] = True
