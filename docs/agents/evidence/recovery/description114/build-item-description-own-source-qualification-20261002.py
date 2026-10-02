"""Read-only bulk qualification of exact own Wiki flavor / pinned XML description."""
import collections
import hashlib
import importlib.util
import json
import re
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

A = Path('/workspace/audit-continuation')
R = Path('/workspace/pr1437-timed-fields')
H = '4f690c1e8a8966a4d074dde94d5a884dbb34fb94'
CUT = '2026-09-27T23:59:59Z'
inputs = {}
def sha(b): return hashlib.sha256(b).hexdigest()
def gitbytes(p):
    b = subprocess.check_output(['git', 'show', H+':'+p], cwd=R)
    inputs[p] = {'sha256': sha(b), 'bytes': len(b), 'source': 'ACTUAL_PUBLISHED_GIT_BLOB'}
    return b
def gj(p): return json.loads(gitbytes(p))
def leaf(d, path):
    v=d.get('semantics', {})
    for k in path.split('.'):
        if 'state' in v:
            if v['state']!='KNOWN': return v
            v=v['value']
        v=v.get(k, {'state':'UNKNOWN'})
    return v
def manifest(p):
    b=p.read_bytes(); return {'path':str(p),'sha256':sha(b),'bytes':len(b)}

sys.path.insert(0, str(R/'tools/content-schema/item-authoring'))
import engine_items as engine
assert (R/'tools/content-schema/item-authoring/engine_items.py').read_bytes()==gitbytes('tools/content-schema/item-authoring/engine_items.py')
spec=importlib.util.spec_from_file_location('own_parser', A/'forge-own-infobox-literal-parser-v2.py')
parser=importlib.util.module_from_spec(spec); spec.loader.exec_module(parser)
print('parser entrypoints', [x for x in dir(parser) if not x.startswith('_')])

wiki_stats=gj('imports/tibiawiki/facts/items-stats.json')['records']
records=gj('content/world/definitions/reference.json')['records']
native={int(x['identity']['key'].rsplit('i',1)[1]):x for x in records if x.get('kind')=='Item'}
assert len(native)==34031
bindings=gj('imports/crystalserver/bindings/items.json')['bindings']
byid=collections.defaultdict(list); bytarget=collections.defaultdict(list)
for x in bindings:
    byid[int(x['external_id'])].append(x); bytarget[x['target']['key']].append(x)
aliases=gj('content/items/aliases.json')['entries']
aliasmap=collections.defaultdict(list)
for x in aliases:
    if x['state']=='ALIAS': aliasmap[x['target']].append(x)
owners=gj('content/world/definitions/declarations.json')['item_authoring']
ownermap=collections.defaultdict(list)
for x in owners: ownermap[x['item']['key']].append(x)
routed=set()
worldpaths=subprocess.check_output(['git','ls-tree','-r','--name-only',H,'content/world/objects','content/world/terrain'],cwd=R).decode().splitlines()
for path in worldpaths:
    if not path.endswith('.json'): continue
    for x in gj(path).get('records',[]):
        p=x.get('provenance',{}).get('item_pointer')
        if p: routed.add(p['key'])
clientsha='2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2'
client=gitbytes('content/assets/files/appearances-'+clientsha+'.dat')
assert sha(client)==clientsha
official={}
for tag,raw in engine.protobuf_fields(client):
    if tag==1:
        obj=engine.decode_appearance_object(raw)
        assert obj['id'] not in official
        official[obj['id']]=obj|{'raw_sha256':sha(raw)}
memberfiles=gj('imports/official/appearance-membership/admitted.json')['files']
members={}
for x in memberfiles:
    members[x['label']]={row[0]:row[2] for row in gj('imports/official/appearance-membership/'+x['manifest'])['entries']}
assert len(official)==43516
historical_raw={}
for label,path in [('crystal-ff7ede5',A/'sources/crystal-ff7ede5-appearances.dat'),('crystal-donor-00ce02a5',A/'sources/crystal-00ce02a5-appearances.dat')]:
    blob=path.read_bytes(); expected=next(x['appearances_sha256'] for x in memberfiles if x['label']==label)
    assert sha(blob)==expected
    values={}
    for tag,raw in engine.protobuf_fields(blob):
        if tag==1:
            fields=engine.protobuf_fields(raw)
            iid=next(v for t,v in fields if t==1)
            assert iid not in values
            values[iid]=sha(raw)
    assert values==members[label]
    historical_raw[label]=manifest(path)

xmlpaths={
 'crystal_ff7':A/'sources/crystal-ff7ede5-items.xml',
 'crystal_00ce':A/'sources/crystal-00ce02a5-items.xml',
 'canary_47':A/'canary-weapon-av-47df/data/items/items.xml',
}
xml={};xmlwitness={}
for label,path in xmlpaths.items():
    if label in ('crystal_ff7','canary_47'):
        sample='crystal-ff7ede5' if label=='crystal_ff7' else 'canary-47dfd51f'
        declared=gj('tools/content-schema/item-authoring/samples/population-'+sample+'.json')['source']['artifact_digests']['data/items/items.xml']['sha256']
        assert sha(path.read_bytes().replace(b'\r\n',b'\n'))==declared
    xml[label]=collections.defaultdict(list)
    for ordinal,node in enumerate(ET.parse(path).getroot()):
        ids=[int(node.attrib['id'])] if 'id' in node.attrib else range(int(node.attrib['fromid']),int(node.attrib['toid'])+1)
        desc=[n.attrib.get('value') for n in node.findall('attribute') if n.attrib.get('key','').lower()=='description']
        if not desc: continue
        witness={'source_file':manifest(path),'element_ordinal':ordinal,'item_attributes':node.attrib,'description_values':desc,'parsed_subtree_not_raw_source':ET.tostring(node,encoding='unicode')}
        for iid in ids:xml[label][iid].append(witness)

allown=collections.defaultdict(list);candidate=set();parts=[]
for path in sorted(A.glob('global-infobox-object-source-part-*.json')):
    raw_part=path.read_bytes();part=json.loads(raw_part);part_hash=sha(raw_part);parts.append(manifest(path))
    assert part['status']==200
    for po,page in enumerate(part['pages']):
        for bi,box in enumerate(page['infobox_objects']):
            fields=collections.defaultdict(list)
            rawparse_error=None
            try:
                parsed=parser.parts(box['raw'][2:-2],'|')
                if re.sub(r'[_\s]+',' ',parsed[0].strip()).casefold()!='infobox object':raise ValueError('NOT_OWN_OBJECT')
                for token in parsed[1:]:
                    key,sep,value=token.partition('=')
                    if sep:
                        key=key.strip().casefold()
                        if not re.fullmatch(r'[a-z0-9_]+',key):raise ValueError('AMBIGUOUS_PARAMETER_KEY')
                        fields[key].append(value.strip())
                captured=collections.defaultdict(list)
                for p in box['parameters']:captured[p['name'].strip().casefold()].append(p['value'].strip())
                if dict(fields)!=dict(captured):raise ValueError('RAW_PARAMETER_CAPTURE_MISMATCH')
            except ValueError as exc:
                rawparse_error=str(exc)
                fields=collections.defaultdict(list)
                for p in box['parameters']:fields[p['name'].strip().casefold()].append(p['value'].strip())
            ids={int(s) for v in fields.get('itemid',[]) for s in re.findall(r'\d+',v)}
            for iid in ids:
                allown[iid].append({'source_part':path.name,'source_part_sha256':part_hash,'source_page_ordinal':po,'page_id':page['page_id'],'title':page['title'],'revision_id':page['revision_id'],'revision_timestamp':page['revision_timestamp'],'article_content_sha256_declared':page['content_sha256'],'box_index':bi,'raw_infobox':box['raw'],'raw_infobox_sha256':sha(box['raw'].encode()),'balanced':box['balanced'],'inside_comment':box['inside_comment'],'positive_exact_infobox_object_match':box['positive_exact_infobox_object_match'],'fields':dict(fields),'independent_raw_parse_error':rawparse_error})
                if any(fields.get('flavortext',[])): candidate.add(iid)

qualified=[];held=[];partial=collections.Counter()
worldflags=('flags.clip','flags.bank','flags.ground','flags.border','flags.bottom','flags.top','flags.corpse','flags.player_corpse','flags.liquidpool','flags.unmove')
for iid in sorted(candidate):
    errs=[]; d=native.get(iid); obj=official.get(iid); src=allown[iid]; values=[]
    if not d: errs.append('NO_CURRENT_NATIVE_ITEM')
    key=d['identity']['key'] if d else 'oteryn:item.tibia.i'+str(iid)
    b=byid[iid]
    if len(b)!=1 or len(bytarget[key])!=1 or b[0]['disposition']!='EXACT' or b[0]['source_key']!='oteryn:source.crystalserver' or b[0]['source_revision'] not in ('ff7ede593c69d4c658b382c97443e8155926924a','00ce02a57ca5a12e48f32a3476e37471167e4c3f') or b[0]['identity_namespace']!='ots/item_server_id' or b[0]['target']!=(d or {}).get('identity'):errs.append('FULL_EXACT_FORWARD_REVERSE_BINDING_HOLD')
    if d and not aliasmap[key]:errs.append('NO_CURRENT_CANONICAL_ALIAS_BRIDGE')
    if not obj: errs.append('NO_CURRENT_OFFICIAL_OBJECT')
    elif obj.get('flags',{}).get('flags.take') is not True or any(obj.get('flags',{}).get(x) is True for x in worldflags):errs.append('PORTABLE_ITEM_DOMAIN_HOLD')
    if key in routed:errs.append('CURRENT_WORLD_OWNER')
    if obj and members['client-15.30'].get(iid)!=obj['raw_sha256']:errs.append('CURRENT_MEMBERSHIP_RAW_HASH_HOLD')
    if len(b)==1:
        lab='crystal-ff7ede5' if b[0]['source_revision'].startswith('ff7ede') else 'crystal-donor-00ce02a5'
        if iid not in members.get(lab,{}):errs.append('EXACT_BOUND_HISTORICAL_MEMBERSHIP_HOLD')
    if d:
        name=leaf(d,'presentation.name');desc=leaf(d,'presentation.description')
        if name.get('state')!='KNOWN' or not obj or name.get('value','').strip().casefold()!=obj.get('name','').strip().casefold():errs.append('STRICT_CURRENT_KNOWN_OFFICIAL_NAME_HOLD')
        if desc!={'state':'UNKNOWN'}:errs.append('DESCRIPTION_ALREADY_KNOWN_OR_BLOCKED')
        if any(x.get('presentation') is not None for x in ownermap[key]):errs.append('AUTHORING_PRESENTATION_OVERRIDE')
    imported_obs=(wiki_stats.get(key) or wiki_stats.get(str(iid)) or {}).get('observations',[])
    if not imported_obs:errs.append('NO_IMPORTED_WIKI_OWN_OBSERVATION')
    for observation in imported_obs:
        matched=[w for w in src if w['page_id']==observation['page_id']]
        if not matched or any(any(observation.get(a)!=w.get(b) for a,b in (('revision_id','revision_id'),('revision_timestamp','revision_timestamp'),('content_sha256','article_content_sha256_declared'),('wiki_title','title'))) for w in matched):errs.append('CURRENT_IMPORTED_WIKI_COORDINATE_DRIFT')
    for s in src:
        f=s['fields']; names=f.get('actualname') if any(f.get('actualname',[])) else f.get('name',[])
        if s['independent_raw_parse_error']:errs.append('RAW_PARAMETER_PARSE_OR_CAPTURE_MISMATCH')
        if not s['balanced'] or s['inside_comment'] or not s['positive_exact_infobox_object_match']:errs.append('INVALID_OR_COMMENTED_OWN_BOX')
        if s['revision_timestamp']>CUT:errs.append('OWN_SOURCE_AFTER_CUTOFF')
        if f.get('itemid')!=[str(iid)]:errs.append('SHARED_OR_AMBIGUOUS_OWN_ID')
        if any(len(f.get(k,[]))>1 for k in ('itemid','actualname','name','flavortext','primarytype')):errs.append('DUPLICATE_OWN_PARAMETER')
        if len(names)!=1 or not obj or names[0].strip().casefold()!=obj.get('name','').strip().casefold():errs.append('WHOLE_GLOBAL_OWN_NAME_DISAGREEMENT')
        if 'flavortext' in f:
            if len(f['flavortext'])!=1 or not f['flavortext'][0]:errs.append('PRESENT_EMPTY_OR_DUPLICATE_FLAVOR')
            else:values.append(f['flavortext'][0])
    if len(set(values))!=1:errs.append('GLOBAL_OWN_FLAVOR_OPPOSITION')
    value=values[0] if len(set(values))==1 else None
    if value:
        if len(value.encode('utf-8'))>200:errs.append('UTF8_OVER_NATIVE_200_BYTE_BOUND')
        if any(tok in value for tok in ('{{','}}','[[',']]','<!--','-->','<','>','&','\n','\r')) or "''" in value:errs.append('MARKUP_OR_MULTILINE_SEMANTICS_UNRESOLVED')
    matching=[]; opposing=[]; xw=[]
    for label,index in xml.items():
        for witness in index.get(iid,[]):
            xw.append({'source_label':label,**witness})
            if witness['description_values']==[value]: matching.append(label)
            else:opposing.append(label)
    if not matching:errs.append('NO_PINNED_OTS_EXACT_DESCRIPTION_CORROBORATION')
    if opposing:errs.append('PRESENT_PINNED_OTS_DESCRIPTION_OPPOSITION')
    out={'source_item_id':iid,'target':(d or {}).get('identity'),'description':value,'utf8_bytes':len(value.encode()) if value else None,'native_classification_state':leaf(d or {},'classification')['state'],'binding':b[0] if len(b)==1 else b,'official_name':obj.get('name') if obj else None,'official_object_sha256':obj.get('raw_sha256') if obj else None,'current_native_name':leaf(d or {},'presentation.name'),'current_alias_rows':aliasmap[key],'current_imported_wiki_observations':imported_obs,'all_own_witnesses':src,'xml_description_witnesses':xw,'matching_pinned_ots_sources':sorted(set(matching)),'holds':sorted(set(errs))}
    if errs:held.append(out)
    else:qualified.append(out)
    if value and not opposing and matching:partial['exact_wiki_xml_description_ids']+=1
out={'schema':'OTERYN_ITEM_DESCRIPTION_OWN_SOURCE_QUALIFICATION_PROPOSAL/v1','status':'SOURCE_QUALIFICATION_NOT_APPLIED','published_native_head':H,'qualification_cutoff':CUT,'native_carrier':'ReferenceItemSemantics.presentation.description = Known(String)','formal_source_properties':['/item/presentation/flavor_text','/item/presentation/inspection_description'],'parser_checks':parser.tests(),'source_policy':'Own numeric-ID Wiki in-game flavortext is the source authority; exact XML description corroborates, never majority or OTS-alone authority. No Wiki notes, name-derived descriptions, markup evaluation, runtime claims, family/admission changes or new ABI. Missing flavor on another own page is not a negative observation; explicit empty, opposing, shared-ID and unsupported markup stay held.','counts':{'current_native_items':len(native),'current_known_names':sum(leaf(x,'presentation.name')['state']=='KNOWN' for x in native.values()),'current_known_descriptions':sum(leaf(x,'presentation.description')['state']=='KNOWN' for x in native.values()),'global_nonempty_flavor_id_discovery':len(candidate),'source_qualified_descriptions':len(qualified),'held_discovery_ids':len(held),**partial},'qualified_ids':[r['source_item_id'] for r in qualified],'qualified':qualified,'held':held,'hold_counts':dict(collections.Counter(x for r in held for x in r['holds'])),'git_inputs':inputs,'source_parts':parts,'xml_source_files':[manifest(p) for p in xmlpaths.values()],'full_historical_membership_raw_readback':historical_raw,'own_flavor_look_contract':{'path':str(A/'stack-forge-maxima-public-20261002/imbuement-renderers-own-exceptions-browser-cdp-source.json'),'page_id':13623,'revision_id':1204205,'revision_timestamp':'2026-08-31T00:49:56Z','evidence':'Template:Infobox Object line301 passes own flavortext unchanged to Infobox Item/Look twbox-look. No notes/quest guide imported.'},'capture_transport':'RETAINED_REMOTE_DESKTOP_CHROME_CDP_PUBLIC_SAME_ORIGIN_API_AFTER_NORMAL_HTTP_BLOCK; current subtask local read-only; declared full article SHA is not independently available article-body proof for ownbox frames','existing_native_codec':'reference_artifact.rs encodes and decodes the existing presentation.description String; no layout addition proposed'}
wiki_only=[r for r in held if set(r['holds'])<={'NO_PINNED_OTS_EXACT_DESCRIPTION_CORROBORATION','PRESENT_PINNED_OTS_DESCRIPTION_OPPOSITION'}]
punctuation=[r for r in wiki_only if r['description'].endswith('.') and r['xml_description_witnesses'] and all(w['description_values']==[r['description'][:-1]] for w in r['xml_description_witnesses'])]
out['additional_policy_review_only']={'otherwise_closed_explicit_wiki_literals':len(wiki_only),'punctuation_only_xml_difference':len(punctuation),'punctuation_only_ids':[r['source_item_id'] for r in punctuation],'remaining_xml_absence_or_other_difference_ids':[r['source_item_id'] for r in wiki_only if r not in punctuation],'status':'NOT_IN_STRICT114_COHORT_NOT_NATIVE_ADMISSION','policy':'No punctuation normalization: Wiki incoming string stays byte-exact. Pinned XML verbatim output differs, requiring explicit independent Wiki source-authority policy decision rather than claimed equality.'}
p=A/'item-description-own-source-qualification-forge4f-20261002.json';p.write_text(json.dumps(out,ensure_ascii=False,indent=2)+'\n');print(out['counts']);print(out['hold_counts']);print(p,sha(p.read_bytes()))
