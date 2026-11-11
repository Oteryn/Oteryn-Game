"""Exact Northern Fishburger custody qualification; no cooking or food-use activation."""
import copy
import hashlib
import json
import qengine_reward_item_profiles as items

PROOF = 'tools/content-schema/native-gameplay/questpc7-hot-fishburger-profile.json'


def capture(root):
    xml_raw = (root/items.XML).read_bytes(); client = (root/items.CLIENT).read_bytes()
    if hashlib.sha256(xml_raw).hexdigest() != items.XML_SHA or hashlib.sha256(client).hexdigest() != items.CLIENT_SHA:
        raise ValueError('QUESTPC7_FISHBURGER_SOURCE_DRIFT')
    source = items.source; xml = source.xml_index(xml_raw)
    objects = source.engine_items.load_appearance_objects(client)
    physical = source.policies(client, xml_raw, {9088})[0]
    bindings = source.binding_index((root/'imports/crystalserver/bindings/items.json').read_bytes())
    record = source.profile(9088, xml[9088], objects[9088], physical, bindings, None)
    record = source.bind_profiles({'schema':'OTERYN_NATIVE_ITEM_PROFILES/v1','records':[record]}, bindings, {9088})['records'][0]
    definition = next(r['definition'] for path in (root/'content/items/definitions').glob('items-*.json') for r in json.loads(path.read_bytes())['records'] if r['definition']['identity']['key'] == 'oteryn:item.tibia.i9088')
    canonical = definition['semantics']; observed = record['semantics']
    known = lambda value: {'state':'KNOWN','value':value}
    if (canonical['physical']['value']['weight'], observed['physical']['value']['weight']) != (known(460), known(459)):
        raise ValueError('QUESTPC7_FISHBURGER_EXACT_WEIGHT_DECISION_DRIFT')
    text = "It's only suited for those who like maritime food"
    if (canonical['presentation']['value']['description'], observed['presentation']['value']['description']) != (known(text+'.'), known(text)):
        raise ValueError('QUESTPC7_FISHBURGER_EXACT_PRESENTATION_DECISION_DRIFT')
    observed['physical']['value']['weight'] = copy.deepcopy(canonical['physical']['value']['weight'])
    observed['presentation'] = copy.deepcopy(canonical['presentation'])
    if any(items.accepted.known_conflict(canonical[group], observed[group]) for group in ('physical', 'stack', 'container', 'presentation')):
        raise ValueError('QUESTPC7_FISHBURGER_CANONICAL_CONFLICT')
    record['semantics'] = items.accepted.accepted_first(canonical, observed)
    return {'schema':'OTERYN_QUESTPC7_HOT_FISHBURGER_PROFILE/v1', 'sources':[{'path':items.XML,'line':24261,'sha256':items.XML_SHA},{'path':items.CLIENT,'sha256':items.CLIENT_SHA}], 'decision':'Retain canonical 460 centi-oz and terminal punctuation over exact donor 459/omitted period; custody only, no food-use effect.', 'records':[record]}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    packet = capture(root)
    generated[PROOF] = (json.dumps(packet,ensure_ascii=False,indent=2)+'\n').encode()
    items.admit_records(root, generated, profile_path, manifest_path, order, canonical, packet['records'])
