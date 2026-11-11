"""Read-only finite native carrying-policy census for every run-8 reward item."""
import collections
import json
from pathlib import Path

keys = set(json.loads(Path('.questpc/r8-catalogue-claims.json').read_text()))
keys.update(json.loads(Path('.questpc/r8-catalogue-key3620-claims.json').read_text()))
native_rows = json.loads(Path('content/items/definitions/spell-native-profiles.json').read_text())['records']
native = collections.defaultdict(list)
for row in native_rows:
    d=row['production_definition']
    native[(d['family'],d['production_key'],d['revision_ref'])].append(row)

def known(value):
    return value.get('value') if isinstance(value,dict) and value.get('state')=='KNOWN' else None

uses=collections.defaultdict(list)
claims={}
issues=[]
for path in sorted(Path('content/interactions/reward_claims').glob('reward-claim*.json')):
    for row in json.loads(path.read_text())['records']:
        d=row['definition'];key=d['identity']['key']
        if key not in keys:continue
        claims[key]=d
        for i,p in enumerate(d['placements']):
            n=d.get('native_contents',{}).get('placements',[])
            n=n[i] if i<len(n) else None
            items=[(n['root'],1,0,n.get('capacity')),*[(c['item'],c['count'],c.get('key_number',0),None) for c in n['contents']]] if n else [(c['item'],c['count'],int(p['reward']['key_binding'].rsplit('/',1)[-1]) if p['reward'].get('key_binding') and not p['reward'].get('container') else 0,None) for c in p['reward']['items']]
            for j,(item,count,number,capacity) in enumerate(items):
                use={'claim':key,'placement':i,'role':'root' if j==0 else 'child','count':count,'key_number':number,'recipe_capacity':capacity,'reward_claim_path':str(path)}
                uses[(item['family'],item['key'],item['revision'])].append(use)

for definition,refs in sorted(uses.items()):
    rows=native.get(definition,[]);reasons=[]
    facts={}
    if len(rows)!=1:reasons.append('NATIVE_PROFILE_MISSING' if not rows else 'NATIVE_PROFILE_DUPLICATE')
    else:
        row=rows[0];a=row['admission'];s=row['semantics'];physical=known(s.get('physical',{})) or {};stack=known(s.get('stack',{})) or {};container=known(s.get('container',{})) or {}
        facts={'materializable':a['materializable'],'destinations':a['legal_destinations'],'stack_class':a['stack_class'],'weight':known(physical.get('weight',{})),'movable':known(physical.get('movable',{})),'pickupable':known(physical.get('pickupable',{})),'stack_max':known(stack.get('stack_max',{})),'container_capacity':known(container.get('capacity',{}))}
        if not a['materializable'] or 'CharacterInventory' not in a['legal_destinations']:reasons.append('CHARACTER_INVENTORY_NOT_ADMITTED')
        if facts['weight'] is None:reasons.append('PORTABLE_WEIGHT_UNKNOWN')
        if facts['movable'] is not True or facts['pickupable'] is not True:reasons.append('PHYSICAL_PORTABILITY_NOT_PROVEN')
        for use in refs:
            if not 0<use['count']<=100:reasons.append('REWARD_QUANTITY_OUTSIDE_OWNER_RANGE')
            if a['stack_class']=='NonStackable' and use['count']!=1:reasons.append('NONSTACKABLE_REWARD_QUANTITY')
            if a['stack_class']=='StackCapable' and (not facts['stack_max'] or use['count']>facts['stack_max']):reasons.append('STACK_MAX_NOT_PROVEN')
            if use['key_number'] and not 1<=use['key_number']<=65535:reasons.append('KEY_NUMBER_OUT_OF_RANGE')
            if use['recipe_capacity'] is not None and facts['container_capacity']!=use['recipe_capacity']:reasons.append('ROOT_CONTAINER_CAPACITY_MISMATCH')
    if reasons:issues.append({'definition':{'family':definition[0],'production_key':definition[1],'revision_ref':definition[2]},'reasons':sorted(set(reasons)),'facts':facts,'uses':refs})
result={'selected_claims':len(keys),'definitions_found':len(claims),'distinct_items':len(uses),'native_profile_rows':len(native_rows),'issues':issues,'items':[{'definition':{'family':d[0],'production_key':d[1],'revision_ref':d[2]},'uses':r} for d,r in sorted(uses.items())]}
Path('.questpc/r8-catalogue-native-audit.json').write_text(json.dumps(result,indent=2)+'\n')
print('claims',len(keys),'definitions',len(claims),'items',len(uses),'issue definitions',len(issues))
for row in issues:print(row['definition']['production_key'],','.join(row['reasons']),'claims',','.join(sorted({u['claim'] for u in row['uses']})))
