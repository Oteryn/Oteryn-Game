"""Source-qualified Marsh Stalker corpse chain using existing Item lifetime owners."""
import hashlib
import json
import qengine_reward_item_profiles as items

IDS = {17428, 17439, 17440, 17441}
CHAIN = {17428:(10,17439,15),17439:(300,17440,15),17440:(300,17441,5),17441:(60,0,None)}
PROOF = 'tools/content-schema/native-gameplay/questpc7-marsh-corpse-profiles.json'


def capture(root):
    xml_raw=(root/items.XML).read_bytes();client=(root/items.CLIENT).read_bytes()
    if hashlib.sha256(xml_raw).hexdigest()!=items.XML_SHA or hashlib.sha256(client).hexdigest()!=items.CLIENT_SHA:
        raise ValueError('QUESTPC7_MARSH_CORPSE_SOURCE_DRIFT')
    source=items.source;xml=source.xml_index(xml_raw)
    objects=source.engine_items.load_appearance_objects(client)
    physical={r['server_item_id']:r for r in source.policies(client,xml_raw,IDS)}
    bindings=source.binding_index((root/'imports/crystalserver/bindings/items.json').read_bytes())
    records=[]
    for item in sorted(IDS):
        a=source.attrs(xml[item]);duration,target,capacity=CHAIN[item]
        if (int(a['duration']),int(a['decayto']),int(a['containersize']) if 'containersize' in a else None)!=(duration,target,capacity):
            raise ValueError('QUESTPC7_MARSH_CORPSE_CHAIN_DRIFT')
        if not objects[item]['flags'].get('flags.corpse'):
            raise ValueError('QUESTPC7_MARSH_CORPSE_FLAG_DRIFT')
        records.append(source.profile(item,xml[item],objects[item],physical[item],bindings,None))
    records=source.bind_profiles({'schema':'OTERYN_NATIVE_ITEM_PROFILES/v1','records':records},bindings,IDS)['records']
    definitions={r['definition']['identity']['key']:r['definition'] for path in (root/'content/items/definitions').glob('items-*.json') for r in json.loads(path.read_bytes())['records']}
    for record in records:
        definition=definitions[record['production_definition']['production_key']]
        if definition['kind']!='Item' or definition['stack_class'] not in ('Unknown',record['admission']['stack_class']):
            raise ValueError('QUESTPC7_MARSH_CORPSE_CANONICAL_IDENTITY_DRIFT')
        semantics=definition.get('semantics')
        if semantics:
            if any(items.accepted.known_conflict(semantics[k],v) for k,v in record['semantics'].items()):raise ValueError('QUESTPC7_MARSH_CORPSE_CANONICAL_FACT_CONFLICT')
            record['semantics']=items.accepted.accepted_first(semantics,record['semantics'])
    return {'schema':'OTERYN_QUESTPC7_MARSH_CORPSE_PROFILES/v1',
        'sources':[{'path':items.XML,'lines':[35365,35419,35424,35429],'sha256':items.XML_SHA},{'path':items.CLIENT,'sha256':items.CLIENT_SHA}],
        'scope':'Exact native corpse decay chain; WorldObject fluid source unchanged.', 'records':records}


def admit(root,generated,profile_path,manifest_path,order,canonical):
    packet=capture(root)
    generated[PROOF]=(json.dumps(packet,ensure_ascii=False,indent=2)+'\n').encode()
    items.admit_records(root,generated,profile_path,manifest_path,order,canonical,packet['records'])
