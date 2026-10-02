import json,pathlib,hashlib,subprocess,sys,re,collections,xml.etree.ElementTree as ET
A=pathlib.Path('/workspace/audit-continuation');R=pathlib.Path('/workspace/pr1437-world-owner-completion');parent='4f690c1e8a8966a4d074dde94d5a884dbb34fb94'
H=lambda b:hashlib.sha256(b).hexdigest();J=lambda p:json.loads(pathlib.Path(p).read_bytes());git=lambda p:subprocess.check_output(['git','show',parent+':'+p],cwd=R)
mp=A/'item-description-source-research-stopped-manifest-forge4f-20261002.json';m=J(mp);assert H(mp.read_bytes())=='77556f5025adab941a86f9a17d4c9221e7cd7f51fc7b8f3c0da3649e7d29b000'
for x in m['owned_files']:
 b=pathlib.Path(x['path']).read_bytes();assert H(b)==x['sha256'] and len(b)==x['bytes']
q=J(A/'item-description-closed114-source-proposal-forge4f-20261002.json');assert H((A/'item-description-closed114-source-proposal-forge4f-20261002.json').read_bytes())=='328ada9dc206b4204bd61f463a5e21e70b473b0d20d4f1c48941aeff04336765'
proposal=J(A/'description1515-explicit-wiki-literal-policy-proposal.json'); assert H((A/'description1515-explicit-wiki-literal-policy-proposal.json').read_bytes())=='c798c1cbce3f35d8b0ab9652cfa4f0fbc0cdc3cebcba863f940fe9923c269f6a'; q['records']=proposal['records']; assert len(q['records'])==1515
for p,w in q['git_inputs'].items():
 b=git(p);assert H(b)==w['sha256'] and len(b)==w['bytes']
raw=git('content/world/definitions/reference.json');items=[x for x in json.loads(raw)['records']if x['kind']=='Item'];native={x['identity']['key']:x for x in items}
leaf=lambda r,k:r.get('semantics',{}).get('presentation',{}).get('value',{}).get(k,{'state':'UNKNOWN'})
assert len(items)==34031 and sum(leaf(x,'name').get('state')=='KNOWN'for x in items)==12100 and sum(leaf(x,'description').get('state')=='KNOWN'for x in items)==0
cr=json.loads(git('imports/crystalserver/bindings/items.json'))['bindings'];aliases=json.loads(git('content/items/aliases.json'))['entries'];stats=json.loads(git('imports/tibiawiki/facts/items-stats.json'))['records'];owners={x['item']['key']:x for x in json.loads(git('content/world/definitions/declarations.json'))['item_authoring']}
world=set()
for p in q['git_inputs']:
 if p.startswith(('content/world/objects/','content/world/terrain/')):
  for x in json.loads(git(p)).get('records',[]):
   ptr=x.get('provenance',{}).get('item_pointer')
   if ptr:world.add(ptr['key'])
sys.path.insert(0,str(R/'tools/content-schema/item-authoring'))
from engine_items import protobuf_fields,decode_appearance_object
from lower_wiki_stack_default_packet import raw_parameters
assert H((R/'tools/content-schema/item-authoring/source_field_catalogs.py').read_bytes())=='8250880aaa89b7c5fa1593d5738a478b0d09e1ef9ef9cef630cb5da172d634bc'
assert H((R/'tools/content-schema/item-authoring/engine_items.py').read_bytes())=='28644c0fcc88f364992670489f7d458f827a9cd1db023b7f5ce0a9ae32d08c1a'
ad=json.loads(git('imports/official/appearance-membership/admitted.json'));official={};data=git('content/assets/files/appearances-2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.dat');assert H(data)=='2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2'
for tag,b in protobuf_fields(data):
 if tag==1:
  obj=decode_appearance_object(b);assert obj['id']not in official;official[obj['id']]=(obj,H(b))
assert len(official)==43516
members={}
for x in ad['files']:
 b=git('imports/official/appearance-membership/'+x['manifest']);assert H(b)==x['manifest_sha256'];members[x['label']]={v[0]:v for v in json.loads(b)['entries']}
for label,w in q['full_historical_membership_raw_readback'].items():
 b=pathlib.Path(w['path']).read_bytes();assert H(b)==w['sha256'] and len(b)==w['bytes'];decoded={}
 for tag,payload in protobuf_fields(b):
  if tag==1:
   oid=next(v for n,v in protobuf_fields(payload)if n==1);assert oid not in decoded;decoded[oid]=H(payload)
 assert decoded=={oid:v[2]for oid,v in members[label].items()}
ids={x['source_item_id']for x in q['records']};assert len(ids)==1515
parts={};globalown=collections.defaultdict(list)
for w in q['source_parts']:
 b=pathlib.Path(w['path']).read_bytes();assert H(b)==w['sha256'] and len(b)==w['bytes'];p=json.loads(b);assert p['status']==200;parts[pathlib.Path(w['path']).name]=(w['sha256'],p)
 for po,page in enumerate(p['pages']):
  for bi,box in enumerate(page['infobox_objects']):
   captured=collections.defaultdict(list)
   for field in box['parameters']:captured[field['name'].strip().casefold()].append(field['value'].strip())
   mentions={int(s)for value in captured.get('itemid',[])for s in re.findall(r'\d+',value)}
   for iid in mentions&ids:globalown[iid].append((pathlib.Path(w['path']).name,po,bi))
xmlpaths={}
for row in q['records']:
 for x in row['xml_description_witnesses']:xmlpaths[x['source_label']]=x['source_file']
xml={}
for label,w in xmlpaths.items():
 b=pathlib.Path(w['path']).read_bytes();assert H(b)==w['sha256']and len(b)==w['bytes'];tree=ET.fromstring(b);xml[label]=collections.defaultdict(list)
 for ordinal,node in enumerate(tree):
  desc=[x.attrib.get('value')for x in node.findall('attribute')if x.attrib.get('key','').lower()=='description']
  if not desc:continue
  targets=[int(node.attrib['id'])]if'id'in node.attrib else range(int(node.attrib['fromid']),int(node.attrib['toid'])+1)
  for iid in targets:
   if iid in ids:xml[label][iid].append((ordinal,node,desc))
maxlen=0;matches=collections.Counter();presentationNames=collections.Counter()
for row in q['records']:
 iid=row['source_item_id'];target=row['target'];key=target['key'];d=native[key];value=row['description'];obj,osh=official[iid]
 assert target==d['identity']and key not in world and leaf(d,'description')=={'state':'UNKNOWN'}and d['semantics']['presentation']['state']=='KNOWN'
 assert leaf(d,'name')==row['current_native_name']and leaf(d,'name')['state']=='KNOWN'and leaf(d,'name')['value'].strip().casefold()==obj['name'].strip().casefold()
 assert value and len(value.encode())==row['utf8_bytes']<=200 and not any(tok in value for tok in ['{{','}}','[[',']]','<!--','-->','<','>','&','\n','\r',"''"])
 assert len([x for x in cr if x['external_id']==str(iid)])==len([x for x in cr if x['target']==target])==1
 assert row['binding']in cr and row['binding']['target']==target and row['binding']['external_id']==str(iid)and row['binding']['disposition']=='EXACT'and row['binding']['identity_namespace']=='ots/item_server_id'
 assert osh==row['official_object_sha256']==members['client-15.30'][iid][2]and obj['flags']['flags.take']is True
 assert not any(obj['flags'].get(x)is True for x in ['flags.clip','flags.bank','flags.ground','flags.border','flags.bottom','flags.top','flags.corpse','flags.player_corpse','flags.liquidpool','flags.unmove'])
 hist='crystal-ff7ede5'if row['binding']['source_revision'].startswith('ff7ede')else'crystal-donor-00ce02a5';assert iid in members[hist]
 assert not owners.get(key,{}).get('presentation');actualAliases=[x for x in aliases if x['state']=='ALIAS'and x['target']==key];assert actualAliases==row['current_alias_rows']and actualAliases
 assert any(x.get('evidence',{}).get('identity_projection_sha256')==members['client-15.30'][iid][1]and x.get('evidence',{}).get('source_item_id')==iid for x in actualAliases)
 assert {(x['source_part'],x['source_page_ordinal'],x['box_index'])for x in row['all_own_witnesses']}==set(globalown[iid])
 for w in row['all_own_witnesses']:
  digest,part=parts[w['source_part']];assert digest==w['source_part_sha256'];page=part['pages'][w['source_page_ordinal']];box=page['infobox_objects'][w['box_index']]
  for a,b in [('page_id','page_id'),('title','title'),('revision_id','revision_id'),('revision_timestamp','revision_timestamp'),('content_sha256','article_content_sha256_declared')]:assert page[a]==w[b]
  assert page['revision_timestamp']<=q['qualification_cutoff']and box['balanced']and not box['inside_comment']and box['positive_exact_infobox_object_match']and box['raw']==w['raw_infobox']and H(box['raw'].encode())==w['raw_infobox_sha256']
  f=raw_parameters(box['raw']);assert f==w['fields']and f['itemid']==[str(iid)]and f['flavortext']==[value]
  names=f.get('actualname')if any(f.get('actualname',[]))else f.get('name',[]);assert len(names)==1 and names[0].strip().casefold()==obj['name'].strip().casefold()
  assert all(len(f.get(k,[]))<=1 for k in ['itemid','name','actualname','flavortext','primarytype'])
  # Display-title variants are permitted only through explicit actualname and own numeric ID.
  presentationNames[(bool(f.get('actualname')),f.get('name',[None])[0]!=names[0])]+=1
 imported=stats[key]['observations'];assert imported==row['current_imported_wiki_observations']and imported
 for obs in imported:
  match=[x for x in row['all_own_witnesses']if x['page_id']==obs['page_id']];assert match
  for w in match:
   for a,b in [('revision_id','revision_id'),('revision_timestamp','revision_timestamp'),('content_sha256','article_content_sha256_declared'),('wiki_title','title')]:assert obs[a]==w[b]
 expected=[]
 for label,index in xml.items():
  for ordinal,node,desc in index.get(iid,[]):
   assert value.endswith('.') and desc==[value[:-1]];expected.append((label,ordinal,dict(node.attrib),desc))
 actual=[(x['source_label'],x['element_ordinal'],x['item_attributes'],x['description_values'])for x in row['xml_description_witnesses']];assert expected==actual
 assert len(expected)>=2 and row['matching_pinned_ots_sources']==[] and set(row['holds'])=={'NO_PINNED_OTS_EXACT_DESCRIPTION_CORROBORATION','PRESENT_PINNED_OTS_DESCRIPTION_OPPOSITION'};matches[tuple(sorted(set(x[0]for x in expected)))]+=1;maxlen=max(maxlen,len(value.encode()))
full=J(q['parent_whole_cohort_report']['path']);assert H(pathlib.Path(q['parent_whole_cohort_report']['path']).read_bytes())==q['parent_whole_cohort_report']['sha256']and len(full['qualified'])==114 and not {x['source_item_id']for x in full['qualified']}&ids
extra=full['additional_policy_review_only'];assert extra['punctuation_only_xml_difference']==1515 and len(extra['remaining_xml_absence_or_other_difference_ids'])==924 and ids==set(extra['punctuation_only_ids'])and not ids&set(extra['remaining_xml_absence_or_other_difference_ids'])
print(json.dumps({'status':'DESCRIPTION1515_SCOPED_WIKI_LITERAL_POLICY_SOURCE_REPLAY_PASS','items':1515,'description_facts_proposed':1515,'all_global_parts':len(parts),'maximum_UTF8_bytes':maxlen,'pinned_XML_differing_observation_sets':{str(k):v for k,v in matches.items()},'current_descriptions_known':0,'selected_period_difference':1515,'excluded_strict':114,'excluded_absent_other':924,'explicitActualName_displayVariants':{str(k):v for k,v in presentationNames.items()},'no_project_mutations':True},indent=2))
