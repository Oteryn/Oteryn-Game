"""Physical smithhammer admission using the existing source importer; no script activation."""
import hashlib
import json
import qengine_reward_item_profiles as items

PROOF = 'tools/content-schema/native-gameplay/questpc7-smith-item-profile.json'


def capture(root):
    xml_raw=(root/items.XML).read_bytes(); client=(root/items.CLIENT).read_bytes()
    if hashlib.sha256(xml_raw).hexdigest()!=items.XML_SHA or hashlib.sha256(client).hexdigest()!=items.CLIENT_SHA:
        raise ValueError('QUESTPC7_SMITH_SOURCE_DRIFT')
    source=items.source; xml=source.xml_index(xml_raw)
    objects=source.engine_items.load_appearance_objects(client)
    physical=source.policies(client,xml_raw,{3208})[0]
    bindings=source.binding_index((root/'imports/crystalserver/bindings/items.json').read_bytes())
    record=source.profile(3208,xml[3208],objects[3208],physical,bindings,None)
    record=source.bind_profiles({'schema':'OTERYN_NATIVE_ITEM_PROFILES/v1','records':[record]},bindings,{3208})['records'][0]
    definition=next(r['definition'] for path in (root/'content/items/definitions').glob('items-*.json') for r in json.loads(path.read_bytes())['records'] if r['definition']['identity']['key']=='oteryn:item.tibia.i3208')
    if definition['semantics']['physical']['value']['weight']!={'state':'KNOWN','value':6800}:
        raise ValueError('QUESTPC7_SMITH_CANONICAL_WEIGHT_DRIFT')
    for group in ('physical','stack','container'):
        if items.accepted.known_conflict(definition['semantics'][group],record['semantics'][group]):
            raise ValueError('QUESTPC7_SMITH_CANONICAL_CONFLICT:'+group)
    record['semantics']=items.accepted.accepted_first(definition['semantics'],record['semantics'])
    return {'schema':'OTERYN_QUESTPC7_SMITH_ITEM_PROFILE/v1','sources':[{'path':items.XML,'line':7933,'sha256':items.XML_SHA},{'path':items.CLIENT,'sha256':items.CLIENT_SHA}],'scope':'Source physical custody and established canonical weapon facts; source moveevent/weapon scripts are not activated.','records':[record]}


def admit(root,generated,profile_path,manifest_path,order,canonical):
    packet=capture(root)
    generated[PROOF]=(json.dumps(packet,ensure_ascii=False,indent=2)+'\n').encode()
    items.admit_records(root,generated,profile_path,manifest_path,order,canonical,packet['records'])
