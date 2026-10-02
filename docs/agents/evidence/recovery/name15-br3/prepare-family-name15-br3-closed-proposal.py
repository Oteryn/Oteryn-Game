"""Offline qualification proposal only; read exact published blobs, never mutate a WT."""
import ast
import collections
import copy
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import types
import xml.etree.ElementTree as ET

sys.dont_write_bytecode = True
A = Path('/workspace/audit-continuation')
R = Path('/workspace/pr1437-timed-fields')
H = 'b0f91e2883007861f5556c44cce13701bb447283'
T = '88edf552b323cef8cb68cef5b10949fbf0c85424'
W = '3ed537c8b7ab5b9ea9bd26f9de976aaa554a1434'
NAMES = (23577,23583,23589,23596,23605,23609,23619,23624,23638,23641,23644,23656,23659,23662,23665)
BRIDS = (52745,52785,52789)
CLIENT_SHA = '2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2'
OLD_REPORT_SHA = 'b3d8ec12f3ecb0841cb5761dcc7273e9c9cf82c445ffc69e592174582c45efdf'
inputs = {}
sha = lambda b: hashlib.sha256(b).hexdigest()
norm = lambda s: s.strip().casefold()

def rawgit(path, head=H):
    b = subprocess.check_output(['git','show',head+':'+path],cwd=R)
    inputs[head+':'+path] = {'commit':head,'path':path,'sha256':sha(b),'bytes':len(b)}
    return b

def gj(path, head=H):
    return json.loads(rawgit(path,head))

def external(path, expected=None):
    p = Path(path); b = p.read_bytes()
    if expected is not None:
        assert sha(b) == expected, 'retained source seal drift'
    inputs[str(p)] = {'path':str(p),'sha256':sha(b),'bytes':len(b)}
    return b

def dump(path,value):
    Path(path).write_text(json.dumps(value,indent=2,ensure_ascii=False,sort_keys=True)+'\n')

# Executed modules are byte-checked to the exact Git input, not an alternate PYTHONPATH.
MODULE_DIR = R/'tools/content-schema/item-authoring'
sys.path.insert(0,str(MODULE_DIR))
for name in ('engine_items','appearance_membership','lower_wiki_stack_default_packet',
             'lower_wiki_movable_packet','lower_client_market_packet'):
    assert (MODULE_DIR/(name+'.py')).read_bytes() == rawgit(str((MODULE_DIR/(name+'.py')).relative_to(R)))
import engine_items as engine
import appearance_membership as member
import lower_wiki_stack_default_packet as base
import lower_wiki_movable_packet as movable
from lower_client_market_packet import WORLD_FLAGS

old = json.loads(external(A/'family235-source-qualification-after-forge4f-20261002.json',OLD_REPORT_SHA))
assert len(old['records']) == 235
name_sources = {r['source_item_id']:r for r in old['conditional_Name15_source_proposals']}
br_sources = {r['source_item_id']:r for r in old['conditional_BR3_source_proposals']}
assert tuple(sorted(name_sources)) == NAMES and tuple(sorted(br_sources)) == BRIDS

name139_path = 'docs/agents/evidence/OTV2-20261002-item-name-source-qualification-v1.json'
name139 = gj(name139_path)
name139_packet = gj('docs/agents/evidence/OTV2-20261002-item-name-promotion-v1.json')
rawgit('tools/content-schema/item-authoring/lower_item_name_packet.py')
rawgit('apps/game-server/src/content/item_name_promotion.rs')
rawgit('tools/content-migration/item_official_navigation.py',T)
rawgit('tools/content-migration/item_external_family_refinement.py',T)
framepath = name139['global_own_id_frame']['path']
framebytes = rawgit(framepath)
assert sha(framebytes) == name139['global_own_id_frame']['sha256']
frame = json.loads(framebytes)
indexed, pages = movable.own_index(frame)
assert all(not indexed.get(i) for i in NAMES)
old_index = json.loads(external(A/'global-infobox-object-own-itemid-index.json'))
old_manifest = json.loads(external(A/'global-infobox-object-source-manifest.json'))
assert old_manifest['status'] == old_index['status'] == 'COMPLETE_CUTOFF_OWN_ID_INDEX'
assert all(not old_index['by_own_itemid_integer_mention'].get(str(i)) for i in NAMES)
assert {str(i):sorted(pids) for i,pids in indexed.items()} == {
    i:sorted({x['page_id'] for x in rows})
    for i,rows in old_index['by_own_itemid_integer_mention'].items()
}

client = rawgit(f'content/assets/files/appearances-{CLIENT_SHA}.dat')
assert sha(client) == CLIENT_SHA and len(client) == 5017996
objects = {}; raws = {}; nameids = collections.defaultdict(list); top = collections.Counter()
for tag,raw in engine.protobuf_fields(client):
    top[tag] += 1
    if tag != 1:
        continue
    values = list(engine.protobuf_fields(raw))
    ids = [v for n,v in values if n==1]
    assert len(ids) == 1 and isinstance(ids[0],int) and ids[0] not in objects
    obj = engine.decode_appearance_object(raw)
    assert obj['id'] == ids[0]
    objects[obj['id']] = obj; raws[obj['id']] = raw
    if obj.get('name'):
        nameids[norm(obj['name'])].append(obj['id'])
assert len(objects) == 43516
manifest_path = f'imports/official/appearance-membership/appearances-{CLIENT_SHA}.json'
membership = gj(manifest_path)
assert membership['entries'] == member.manifest_entries(client)
assert membership['entries_sha256'] == member.entries_digest(membership['entries'])
admitted = gj('imports/official/appearance-membership/admitted.json')
assert admitted['newest'] == 'client-15.30' and admitted['current_count'] == 43516
assert admitted['files'][-1]['manifest_sha256'] == inputs[H+':'+manifest_path]['sha256']
mem = {v[0]:v for v in membership['entries']}
bindings = gj('imports/crystalserver/bindings/items.json')['bindings']
bykey = base.exact_bindings(bindings,name139['source_revisions'])
all_bykey = collections.defaultdict(list); all_byexternal = collections.defaultdict(list)
for b in bindings:
    all_bykey[b['target']['key']].append(b)
    all_byexternal[(b['source_key'],b['source_revision'],b['identity_namespace'],b['external_id'])].append(b)
index = gj('content/items/index.json'); defs = {}; outer = {}; shardpaths = {}
for path in index['shards']:
    doc = gj(path)
    assert doc['schema'] == 'OTERYN_ITEM_AUTHORING_SHARD/v1' and doc['family'] == 'Item'
    for row in doc['records']:
        d = row['definition']; key = d['identity']['key']
        assert key not in defs
        defs[key] = d; outer[key] = doc; shardpaths[key] = path
assert len(defs) == index['record_count'] == 34031
owners = gj('content/world/definitions/declarations.json')['item_authoring']
assert len(owners) == 411
routed = set(); routed_overlay = set()
for head,route in ((H,routed),(W,routed_overlay)):
    paths = subprocess.check_output(['git','ls-tree','-r','--name-only',head,'content/world/objects','content/world/terrain'],cwd=R).decode().splitlines()
    for path in paths:
        if not path.endswith('.json'): continue
        for row in gj(path,head).get('records',[]):
            pointer = row.get('provenance',{}).get('item_pointer')
            if pointer: route.add(pointer['key'])
aliases = gj('content/items/aliases.json')['entries']
imported = gj('tools/content-schema/item-authoring/samples/promotion-crystal-ff7ede5.json')['promotions']
catalog = gj('tools/content-schema/item-authoring/profile-catalog.json')
assert [p['profile_id'] for p in catalog['profiles'] if 'Itens de Quest' in p['navigation_families']] == ['quest_item']
rawgit('tools/content-schema/item-authoring/source_field_catalogs.py')
helper = rawgit('tools/content-census/item_wiki_family_capture.py',T).decode()
tree = ast.parse(helper)
nodes = [n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name in ('split_template_params','resolve_infobox_fields')]
ns = {'re':re,'engine_items':engine}
exec(compile(ast.Module(body=nodes,type_ignores=[]),'accepted-frozen-family-functions','exec'),ns)
xml = external(A/'sources/crystal-ff7ede5-items.xml','c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb')
matches = list(re.finditer(rb'<item\s+fromid="23577"\s+toid="23667"\s+name="weapon of mayhem"\s*>.*?</item>',xml,re.S))
assert len(matches) == 1
xraw = matches[0].group().decode(); xelem = ET.fromstring(xraw)
assert xelem.attrib == {'fromid':'23577','toid':'23667','name':'weapon of mayhem'}

def current_guard(source):
    iid = source['source_item_id']; target = source['target']; key = target['key']
    binding = source.get('binding',source.get('current_binding'))
    assert target == defs[key]['identity'] and target['family']=='Item' and target['revision']=='definition-r1'
    assert bykey[key] == binding and binding['external_id'] == str(iid)
    assert all_bykey[key] == [binding]
    reverse = (binding['source_key'],binding['source_revision'],binding['identity_namespace'],str(iid))
    assert all_byexternal[reverse] == [binding]
    assert key not in routed and key not in routed_overlay
    assert defs[key]['kind']=='Item'
    same = [o for o in owners if o['item']['key']==key]
    assert len(same)<=1 and all(o['item']==target and o.get('presentation') is None for o in same)
    obj = objects[iid]; fields = list(engine.protobuf_fields(raws[iid]))
    ownname = [v for tag,v in fields if tag==4]
    assert len(ownname)==1 and ownname[0].decode('utf-8','strict')==obj['name']
    assert obj['flags'].get('flags.take') is True
    assert not any(obj['flags'].get(k) is True for k in (*WORLD_FLAGS,'flags.unmove','flags.ground','flags.border','flags.bottom','flags.top'))
    assert mem[iid][2] == sha(raws[iid])
    return {'four_headers':{'shard.schema':outer[key]['schema'],'shard.family':outer[key]['family'],
                           'definition.kind':defs[key]['kind'],'definition.identity.family':target['family']},
            'materializable':defs[key]['materializable'],'stack_class':defs[key]['stack_class'],
            'name':movable.leaf(defs[key],'presentation.name'),
            'native_shard':shardpaths[key],'native_definition_sha256':sha(member.canonical_bytes(defs[key])),
            'binding':binding,'reverse_binding_count':1,'GameOwned_presentation_absent':True,
            'current_World_owner_absent':True,'published_World_overlay_owner_absent':True,
            'official_raw_object_sha256':mem[iid][2],'official_identity_projection_sha256':mem[iid][1],
            'official_name_field_occurrences':1,'official_full_artifact_numeric_identity_unique':True,
            'official_same_name_numeric_ids':sorted(nameids[norm(obj['name'])])}

def imported_name_guard(name,incoming,previous):
    assert previous=='weapon of mayhem' and incoming.strip()
    assert name in ({'state':'KNOWN','value':previous},{'state':'KNOWN','value':incoming})

name_rows = []
for iid in NAMES:
    s = name_sources[iid]; cur = current_guard(s); obj = objects[iid]
    assert obj['name'] == s['official_name'] and sha(raws[iid]) == s['official_object_sha256']
    assert cur['name']=={'state':'KNOWN','value':'weapon of mayhem'}
    imported_name_guard(cur['name'],obj['name'],s['previous_imported_name'])
    expected = s['imported_name_proof']
    assert [r for r in imported if r['field_path']=='presentation.name' and r['source_item_id']==iid]==[expected]
    assert expected['typed_value']=={'kind':'TEXT','value':'weapon of mayhem'} and expected['source_value']=='weapon of mayhem'
    assert [a['target'] for a in aliases if a['key']==expected['native_key'] and a['state']=='ALIAS']==[s['target']['key']]
    assert int(xelem.attrib['fromid'])<=iid<=int(xelem.attrib['toid'])
    frames = copy.deepcopy(s['retained_name_source_page_frames'])
    for pg in frames:
        part = json.loads(external(pg['source_part'],pg['source_part_sha256']))
        found = [p for p in part['pages'] if p.get('page_id')==pg['page_id'] and p.get('revision_id')==pg['revision_id']]
        assert len(found)==1 and found[0]['content_sha256']==pg['content_sha256']
        for box in pg['own_boxes']:
            assert sha(box['raw'].encode())==box['raw_sha256']
            params = base.raw_parameters(box['raw'])
            assert params==box['parameter_values']
            fields = {k:v[0] for k,v in params.items() if v}
            assert list(ns['resolve_infobox_fields'](fields))==box['accepted_resolution']
            actual = fields.get('actualname') or fields.get('name')
            assert norm(actual)==norm(obj['name'])
            assert all(iid!=int(v) for value in params.get('itemid',[]) for v in re.findall(r'\d+',value))
    resolutions = {box['accepted_resolution'][0] for pg in frames for box in pg['own_boxes']}
    assert len(resolutions)==1 and None not in resolutions
    name_rows.append({'source_item_id':iid,'target':s['target'],'facts':{'name':obj['name']},
        'previous_imported_name':'weapon of mayhem','imported_name_proof':expected,'current':cur,
        'own_Wiki_target_ID_set':[],'own_Wiki_authority_for_correction':False,
        'classification_of_raw_name':'PROVEN_OFFICIAL_OWN_OBJECT_FIELD4',
        'correction_qualification':'DERIVED_CLOSED_OFFICIAL_ID_IMPORT_LITERAL_POLICY_EXTENSION_PROPOSED',
        'family_followup':{'profile':next(iter(resolutions)),'scope':'NAVIGATION_ONLY',
            'status':'CONDITIONAL_AFTER_NAME_CORRECTION_AND_ACCEPTED_JOIN_REPLAY',
            'source_frames':frames,'historical_accepted_attempts':s['historical_family_attempts'],
            'whole_article_disambiguation_branch_proven':False,'scalar_inheritance_permitted':False}})

br_rows = []
freshpath = A/'stack-forge-maxima-public-20261002/family-nine-browser-cdp-source.json'
fresh = json.loads(external(freshpath,'b04faaf9bdb1f02ece895334967e306b22241db79b0f7ab25803ab06226cfacb'))
assert fresh['status']==200 and sha(fresh['body'].encode())==fresh['body_sha256']
fresh_pages = {p['pageid']:p for p in json.loads(fresh['body'])['query']['pages'] if 'pageid' in p}
for iid in BRIDS:
    s = br_sources[iid]; cur = current_guard(s); obj = objects[iid]
    assert cur['name']=={'state':'KNOWN','value':obj['name']} and len(nameids[norm(obj['name'])])==1
    expected = {(p['page_id'],p['box_index']) for p in s['all_selected_own_id_frames']}
    actual = {(pid,b['box_index']) for pid in indexed[iid] for b in pages[pid]['own_objects']
              if any(str(iid) in re.findall(r'\d+',v) for v in b['raw_itemid_values'])}
    assert actual==expected and len(actual)==1
    for pg in s['all_selected_own_id_frames']:
        assert pg['revision_timestamp']<=frame['qualification_cutoff']
        fields = base.raw_parameters(pg['raw_infobox']); assert fields==pg['parameter_values']
        assert fields['itemid']==[str(iid)] and fields['primarytype']==['Others'] and fields['pickupable']==['yes']
        assert fields.get('objectclass')==[''] and not any(v for k in ('secondarytype','status') for v in fields.get(k,[]))
        assert norm((fields.get('actualname') or fields['name'])[0])==norm(obj['name'])
        freshpg = fresh_pages[pg['page_id']]; freshrev = freshpg['revisions'][0]
        freshtext = freshrev['slots']['main']['content']
        assert freshrev['revid']==pg['revision_id'] and freshrev['timestamp']==pg['revision_timestamp']
        assert sha(freshtext.encode())==pg['content_sha256'] and pg['raw_infobox'] in freshtext
        full_fandom = {'page_id':pg['page_id'],'revision_id':freshrev['revid'],
            'revision_timestamp':freshrev['timestamp'],'full_article':freshtext,
            'whole_article_sha256':sha(freshtext.encode()),'capture_path':str(freshpath),
            'capture_body_sha256':fresh['body_sha256'],'requested_url':fresh['requested_url']}
    aux = s['auxiliary_BR_page']; cap = json.loads(external(aux['capture_path'],aux['capture_sha256']))
    assert sha(cap['body'].encode())==cap['body_sha256']==aux['body_sha256'] and cap['status']==200
    pg = next(p for p in json.loads(cap['body'])['query']['pages'] if p.get('pageid')==aux['page_id'])
    rev = pg['revisions'][0]; content = rev['slots']['main']['content']
    assert rev['revid']==aux['revision_id'] and rev['timestamp']==aux['revision_timestamp']<=frame['qualification_cutoff']
    assert sha(content.encode())==aux['whole_article_sha256'] and aux['raw_infobox']==content
    assert sha(aux['raw_infobox'].encode())==aux['raw_infobox_sha256']
    # Raw bounded BR frames are Infobox_Item, not the own-Fandom Infobox Object grammar.
    assert re.match(r'^\{\{Infobox_Item\|',aux['raw_infobox']) and '<!--' not in aux['raw_infobox']
    vals = collections.defaultdict(list)
    for term in ns['split_template_params'](aux['raw_infobox'][2:-2])[1:]:
        if '=' in term:
            k,v=term.split('=',1); vals[norm(k)].append(v.strip())
    assert dict(vals)==aux['parameter_values'] and vals['primarytype']==['Itens de Quest']
    assert vals['name']==[pg['title']] and norm(pg['title'])==norm(obj['name'])
    assert not vals.get('itemid') and all(len(vals[k])==1 for k in ('name','primarytype','itemclass'))
    # Preserve every old source opposition; coarse Others is unresolved, not a conflicting admitted family.
    observations = gj('imports/tibiawiki/facts/items-stats.json')['records'].get(str(iid),{}).get('observations',[])
    assert all(ns['resolve_infobox_fields'](o['fields'])[0] in (None,'quest_item') for o in observations)
    br_rows.append({'source_item_id':iid,'target':s['target'],'current':cur,'family_profile':'quest_item',
        'source_classification':'DERIVED','scope':'NAVIGATION_ONLY','own_Fandom_identity_frames':s['all_selected_own_id_frames'],
        'fresh_full_article_Fandom_source':full_fandom,'auxiliary_BR_source':aux,
        'full_article_BR_source':content,'retained_stats_opposition':observations,
        'BR_is_numeric_identity_authority':False,'official_name_global_unique':True,
        'BR_actor_effect_values_imported':False,'accepted_resolver_current_result':None,
        'qualification_status':'CONDITIONAL_CLOSED_AUXILIARY_BR_SOURCE_POLICY_EXTENSION_PROPOSED'})

negative_checks = []
def expect_reject(label,fn):
    try:
        fn()
    except (AssertionError,KeyError):
        negative_checks.append({'case':label,'result':'REJECTED'})
    else:
        raise AssertionError('negative source guard admitted '+label)

for label,mutate in (
    ('wrong fullTarget revision',lambda s:s['target'].__setitem__('revision','definition-r2')),
    ('wrong numeric source ID',lambda s:s.__setitem__('source_item_id',23578)),
    ('wrong identity namespace',lambda s:s['binding'].__setitem__('identity_namespace','legacy/item_id')),
):
    bad=copy.deepcopy(name_sources[NAMES[0]]); mutate(bad)
    expect_reject(label,lambda bad=bad:current_guard(bad))
for state in ('UNKNOWN','CONFLICT','NOT_APPLICABLE'):
    expect_reject('blocked name '+state,lambda state=state:imported_name_guard({'state':state},'blade of mayhem','weapon of mayhem'))
expect_reject('unrelated Known name',lambda:imported_name_guard({'state':'KNOWN','value':'GameOwned name'},'blade of mayhem','weapon of mayhem'))
expect_reject('another historical generic literal',lambda:imported_name_guard({'state':'KNOWN','value':'weapon of carving'},'blade of mayhem','weapon of carving'))
imported_name_guard({'state':'KNOWN','value':'blade of mayhem'},'blade of mayhem','weapon of mayhem')
negative_checks.append({'case':'identical incoming Known name','result':'IDEMPOTENT_ACCEPTED'})

proof = {'schema':'OTERYN_CLOSED_NAME15_AND_AUXILIARY_BR3_SOURCE_PROPOSAL/v1','status':'SOURCE_QUALIFIED_PROPOSAL_NOT_ACCEPTED_OR_APPLIED',
    'baseline':H,'original_family235_baseline':old['native_baseline'],'taxonomy_overlay':T,'World_overlay':W,
    'qualification_cutoff':frame['qualification_cutoff'],'source_access':'OFFLINE_EXACT_GIT_BLOBS_AND_RETAINED_PUBLIC_SOURCE_ONLY',
    'native_changes_applied':0,'family_changes_applied':0,'fully_verified':'NOT_ESTABLISHED',
    'closed_source_item_ids':{'name15':list(NAMES),'BR3':list(BRIDS)},
    'counts':{'name_correction_candidates':15,'nav_followup_after_name':15,'auxiliary_BR_family_candidates':3,'other_family235_holds':217},
    'official_full_source':{'artifact_sha256':CLIENT_SHA,'bytes':len(client),'object_count':43516,
        'unique_numeric_object_ids':43516,'top_level_family_tag_counts':dict(top),
        'membership_exact_regeneration':True,'duplicate_numeric_IDs':0,'unique_name_values_required_for_Name15':False},
    'protected_XML_source':{'source_repository':'zimbadev/crystalserver','source_revision':name139['protected_source_lineage']['source_revision'],
        'path':'data/items/items.xml','whole_raw_sha256':sha(xml),'digest_mode':'text_lf_normalized',
        'raw_range':xraw,'raw_range_sha256':sha(xraw.encode()),'attributes':xelem.attrib,
        'correction_scope':list(NAMES),'range_widening_permitted':False},
    'complete_own_Wiki_identity_priority':{'source_frame_path':framepath,'source_frame_sha256':sha(framebytes),
        'selected_pages':9980,'integer_mention_IDs':len(indexed),'index_matches_retained_census':True,
        'no_own_Wiki_ID_for_all_Name15':True,'own_Fandom_ID_singleton_for_all_BR3':True},
    'owning_contract_decision':{'existing_leaf':'ReferenceItemSemantics.presentation.name',
        'new_model_or_native_group_required':False,'new_source_qualification_policy_exception_required':True,
        'original_Name139_policy_exact_quote':name139['source_policy']['names'],
        'Name139_owned_cohort_is_immutable':True,
        'Name139_witness_params_applies_to_Name15':False,
        'NAV6_authority_scope':'A12 numeric official identity/name supports navigation-only predicates; does not grant generic Known-name overwrite',
        'proposed_exception':'Only the fixed15 exact numeric official own names may replace the exact proven imported KNOWN weapon of mayhem, or accept identical incoming KNOWN idempotently; every other state/literal/target/owner rejects. Own-Wiki-ID absence is explicit and no source boxes are fabricated.',
        'BR3_exception':'Only fixed52745/52785/52789 ownFandom singleton identities and globally unique official names may bridge raw BR Itens de Quest to existing quest_item navigation category; no resolver/catalog alias addition.'},
    'name15':name_rows,'BR3':br_rows,
    'other217_hold_ids':[r['id'] for r in old['records'] if r['id'] not in NAMES+BRIDS],
    'offline_proposal_guard_checks':negative_checks,
    'continuity':'Pinned source/source cutoff proof; public revision continuity beyond captured revisions UNKNOWN',
    'source_input_digests':list(inputs.values())}
assert len(proof['other217_hold_ids'])==217 and len(name_rows)==15 and len(br_rows)==3
out = A/'family-name15-br3-closed-source-proposal-20261002.json'
dump(out,proof)
proposed = {'schema':'OTERYN_UNACCEPTED_CLOSED_SOURCE_PACKET_PROPOSAL/v1','status':'PROPOSAL_ONLY_NOT_EXECUTABLE_PACKET',
    'baseline':H,'source_qualification_sha256':sha(out.read_bytes()),'policy_acceptance_required':True,
    'name15':{'count':15,'promotions':[{'target':r['target'],'source_item_id':r['source_item_id'],
        'previous_imported_name':'weapon of mayhem','field_path':'presentation.name','typed_value':{'kind':'TEXT','value':r['facts']['name']}}
        for r in name_rows]},
    'auxiliary_BR3':{'count':3,'records':[{'target':r['target'],'source_item_id':r['source_item_id'],
        'family_profile':'quest_item','classification':'DERIVED','scope':'NAVIGATION_ONLY',
        'Fandom_page_revision':[(p['page_id'],p['revision_id'])for p in r['own_Fandom_identity_frames']],
        'BR_page_revision':[r['auxiliary_BR_source']['page_id'],r['auxiliary_BR_source']['revision_id']]}for r in br_rows]},
    'family15_followup':{'count':15,'status':'WAIT_CORRECTION_THEN_REAL_ACCEPTED_JOIN_REPLAY',
        'profiles':dict(collections.Counter(r['family_followup']['profile']for r in name_rows))},
    'runtime_or_native_classification_admission':False,'applied':0}
dump(A/'family-name15-br3-proposed-packet-20261002.json',proposed)
print(json.dumps({'status':proof['status'],'name15':15,'BR3':3,'other_holds':217,
                  'family15_profiles':proposed['family15_followup']['profiles'],
                  'proof_sha256':sha(out.read_bytes()),'input_files':len(inputs)}))
