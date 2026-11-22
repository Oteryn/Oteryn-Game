"""Source-qualified Elephant corpse chain using existing Item lifetime owners."""
import hashlib
import json
import qengine_reward_item_profiles as items

CHAIN = {6052:(10,4360,30),4360:(300,4361,30),4361:(300,4362,30),4362:(60,0,None)}
IDS = frozenset(CHAIN)
PROOF = 'tools/content-schema/native-gameplay/elephant-corpse-profiles.json'


def capture_chain(root, chain):
    ids = set(chain)
    xml_raw=(root/items.XML).read_bytes();client=(root/items.CLIENT).read_bytes()
    if hashlib.sha256(xml_raw).hexdigest()!=items.XML_SHA or hashlib.sha256(client).hexdigest()!=items.CLIENT_SHA:
        raise ValueError('ELEPHANT_CORPSE_CORPSE_SOURCE_DRIFT')
    source=items.source;xml=source.xml_index(xml_raw)
    objects=source.engine_items.load_appearance_objects(client)
    physical={r['server_item_id']:r for r in source.policies(client,xml_raw,ids)}
    bindings=source.binding_index((root/'imports/crystalserver/bindings/items.json').read_bytes())
    records=[]
    for item in sorted(ids):
        a=source.attrs(xml[item]);duration,target,capacity=chain[item]
        if (int(a['duration']),int(a['decayto']),int(a['containersize']) if 'containersize' in a else None)!=(duration,target,capacity):
            raise ValueError('ELEPHANT_CORPSE_CORPSE_CHAIN_DRIFT')
        if not objects[item]['flags'].get('flags.corpse'):
            raise ValueError('ELEPHANT_CORPSE_CORPSE_FLAG_DRIFT')
        records.append(source.profile(item,xml[item],objects[item],physical[item],bindings,None))
    records=source.bind_profiles({'schema':'OTERYN_NATIVE_ITEM_PROFILES/v1','records':records},bindings,ids)['records']
    definitions={r['definition']['identity']['key']:r['definition'] for path in (root/'content/items/definitions').glob('items-*.json') for r in json.loads(path.read_bytes())['records']}
    for record in records:
        definition=definitions[record['production_definition']['production_key']]
        if definition['kind']!='Item' or definition['stack_class'] not in ('Unknown',record['admission']['stack_class']):
            raise ValueError('ELEPHANT_CORPSE_CORPSE_CANONICAL_IDENTITY_DRIFT')
        semantics=definition.get('semantics')
        if semantics:
            if any(items.accepted.known_conflict(semantics[k],v) for k,v in record['semantics'].items()):raise ValueError('ELEPHANT_CORPSE_CORPSE_CANONICAL_FACT_CONFLICT')
            record['semantics']=items.accepted.accepted_first(semantics,record['semantics'])
    return records


def capture(root):
    return {'schema':'OTERYN_ELEPHANT_CORPSE_PROFILES/v1',
        'sources':[{'path':items.XML,'lines':[13782,13788,13793,16650],'sha256':items.XML_SHA},{'path':items.CLIENT,'sha256':items.CLIENT_SHA}],
        'scope':'Exact native corpse decay chain; WorldObject fluid source unchanged.', 'records':capture_chain(root, CHAIN)}


def admit(root,generated,profile_path,manifest_path,order,canonical):
    packet=capture(root)
    generated[PROOF]=(json.dumps(packet,ensure_ascii=False,indent=2)+'\n').encode()
    items.admit_records(root,generated,profile_path,manifest_path,order,canonical,packet['records'])
