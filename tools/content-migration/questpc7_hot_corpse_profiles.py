"""Source-qualified Hot Cuisine Bat/Chicken corpse chain using existing Item lifetime owners."""
import hashlib
import json
import qengine_reward_item_profiles as items

IDS = {4330, 4331, 4332, 4363, 4364, 4365, 6042, 6053}
CHAIN = {6042:(10,4330,5),4330:(300,4331,5),4331:(300,4332,None),4332:(60,0,None),6053:(10,4363,14),4363:(300,4364,14),4364:(300,4365,None),4365:(60,0,None)}
PROOF = 'tools/content-schema/native-gameplay/questpc7-hot-corpse-profiles.json'


def capture(root):
    xml_raw=(root/items.XML).read_bytes();client=(root/items.CLIENT).read_bytes()
    if hashlib.sha256(xml_raw).hexdigest()!=items.XML_SHA or hashlib.sha256(client).hexdigest()!=items.CLIENT_SHA:
        raise ValueError('QUESTPC7_HOT_CUISINE_CORPSE_SOURCE_DRIFT')
    source=items.source;xml=source.xml_index(xml_raw)
    objects=source.engine_items.load_appearance_objects(client)
    physical={r['server_item_id']:r for r in source.policies(client,xml_raw,IDS)}
    bindings=source.binding_index((root/'imports/crystalserver/bindings/items.json').read_bytes())
    records=[]
    for item in sorted(IDS):
        a=source.attrs(xml[item]);duration,target,capacity=CHAIN[item]
        if (int(a['duration']),int(a['decayto']),int(a['containersize']) if 'containersize' in a else None)!=(duration,target,capacity):
            raise ValueError('QUESTPC7_HOT_CUISINE_CORPSE_CHAIN_DRIFT')
        if not objects[item]['flags'].get('flags.corpse'):
            raise ValueError('QUESTPC7_HOT_CUISINE_CORPSE_FLAG_DRIFT')
        records.append(source.profile(item,xml[item],objects[item],physical[item],bindings,None))
    records=source.bind_profiles({'schema':'OTERYN_NATIVE_ITEM_PROFILES/v1','records':records},bindings,IDS)['records']
    definitions={r['definition']['identity']['key']:r['definition'] for path in (root/'content/items/definitions').glob('items-*.json') for r in json.loads(path.read_bytes())['records']}
    for record in records:
        definition=definitions[record['production_definition']['production_key']]
        if definition['kind']!='Item' or definition['stack_class'] not in ('Unknown',record['admission']['stack_class']):
            raise ValueError('QUESTPC7_HOT_CUISINE_CORPSE_CANONICAL_IDENTITY_DRIFT')
        semantics=definition.get('semantics')
        if semantics:
            if any(items.accepted.known_conflict(semantics[k],v) for k,v in record['semantics'].items()):raise ValueError('QUESTPC7_HOT_CUISINE_CORPSE_CANONICAL_FACT_CONFLICT')
            record['semantics']=items.accepted.accepted_first(semantics,record['semantics'])
    return {'schema':'OTERYN_QUESTPC7_HOT_CUISINE_CORPSE_PROFILES/v1',
        'sources':[{'path':items.XML,'lines':[13638,13645,13650,13797,13804,13809,16590,16656],'sha256':items.XML_SHA},{'path':items.CLIENT,'sha256':items.CLIENT_SHA}],
        'scope':'Exact native corpse decay chain; WorldObject fluid source unchanged.', 'records':records}


def admit(root,generated,profile_path,manifest_path,order,canonical):
    packet=capture(root)
    generated[PROOF]=(json.dumps(packet,ensure_ascii=False,indent=2)+'\n').encode()
    items.admit_records(root,generated,profile_path,manifest_path,order,canonical,packet['records'])
