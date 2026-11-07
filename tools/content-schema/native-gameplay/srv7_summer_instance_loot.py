"""Closed Summer2026 source loot/body facts; existing loot draws stay unchanged."""
import json
BODIES = frozenset({43758,54346,54352,54433,54670})
LOOT = frozenset({5888, 7424, 32771, 8072, 8074, 43917, 7440, 7441, 3346, 7443, 54269, 3093, 54270, 8090, 8092, 6558, 3360, 5922, 3364, 3369, 816, 22193, 820, 54480, 17850, 39038, 828, 22085, 24392, 54611, 54128, 23373, 23374, 23375, 53197, 53199, 3026, 3027, 3284, 43731, 21974, 21975, 53201, 53203, 53205, 3035, 7643, 3037, 21981, 54640, 3041, 9057, 3043, 6499, 9058, 30053, 3306, 30059, 7404, 237, 238, 3567, 30060, 32624, 53229, 53230, 22516, 53231, 53232, 49271, 53233, 3577, 54651, 54268, 16125, 3070})
def admit(facts, definitions):
    path=definitions/'spell-native-profiles.json'
    if not path.is_file():return
    native=json.loads(path.read_bytes())
    if native['schema']!='OTERYN_NATIVE_ITEM_PROFILES/v1':raise ValueError('Summer source facts require native Item provider')
    rows={r['production_definition']['production_key']:r for r in native['records']}
    if len(rows)!=len(native['records']):raise ValueError('Summer source Item provider duplicates')
    for identifier in BODIES|LOOT:
        key=f'oteryn:item.tibia.i{identifier}'
        fact=facts.get(key);row=rows.get(key)
        if fact is None or row is None:continue
        ref=row['production_definition'];expected={'family':'Item','key':key,'revision':ref['revision_ref']}
        if ref['family']!='Item' or ref['revision_ref']!='definition-r1' or fact['item']!=expected or row['production_binding']['disposition']!='EXACT' or row['production_binding']['target']!=expected:
            raise ValueError('Summer source facts require exact source-qualified binding: '+key)
        admission=row['admission'];sem=row['semantics']
        if admission['materializable'] is not True:continue
        if identifier in BODIES:
            if admission!={'materializable':True,'stack_class':'NonStackable','legal_destinations':['Ground']} or sem.get('container')!={'state':'KNOWN','value':{'capacity':{'state':'KNOWN','value':30}}}:continue
            fact['materializable']=True;fact['container_capacity']=30
            continue
        physical=sem.get('physical',{})
        if 'CharacterInventory'not in admission['legal_destinations'] or physical.get('state')!='KNOWN' or physical['value']['movable']!={'state':'KNOWN','value':True} or physical['value']['pickupable']!={'state':'KNOWN','value':True}:continue
        fact['materializable']=True
        if sem.get('stack')=={'state':'KNOWN','value':{'stackable':{'state':'KNOWN','value':True},'stack_max':{'state':'KNOWN','value':100}}} and admission['stack_class']=='StackCapable':fact['stackable']=True
        container=sem.get('container',{})
        if container.get('state')=='KNOWN':
            capacity=container['value']['capacity']
            if capacity.get('state')=='KNOWN' and 1<=capacity['value']<=500:fact['container_capacity']=capacity['value']
