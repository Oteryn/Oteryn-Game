"""Stopped read-only source qualification: all235 + conditional imported Name15 lane."""
from pathlib import Path
import ast,collections,hashlib,json,re,struct,subprocess,sys,types,xml.etree.ElementTree as ET
sys.dont_write_bytecode=True
A=Path('/workspace/audit-continuation');R=Path('/workspace/pr1437-stack-publish');H='4f690c1e8a8966a4d074dde94d5a884dbb34fb94';T='88edf552b323cef8cb68cef5b10949fbf0c85424';w=[]
sha=lambda b:hashlib.sha256(b).hexdigest()
def rd(p):
 p=Path(p);b=p.read_bytes();w.append({'path':str(p),'sha256':sha(b),'bytes':len(b)});return json.loads(b)
def rawgt(p,head=H):
 b=subprocess.check_output(['git','show',head+':'+p],cwd=R);w.append({'path':p,'commit':head,'sha256':sha(b),'bytes':len(b)});return b
def gt(p,head=H):return json.loads(rawgt(p,head))
def norm(s):return (s or '').strip().casefold()
old=rd(A/'final-item-classification-235.json');inv=rd(A/'item-closure-inventory-88edf552-8257ba68-world3ed537c8.compact.json');status={n:k for k,v in inv['classification_item_ids'].items() for n in v};ids={r['id'] for r in old['records']};assert len(ids)==235 and ids=={n for n,s in status.items() if s!='CATEGORIZED'}
source=rawgt('tools/content-schema/item-authoring/engine_items.py').decode();tree=ast.parse(source);ns={'re':re,'struct':struct};names={'PRIMARYTYPE_PROFILE','WIKI_OBJECTCLASS_PROFILE','WIKI_STATUS_PROFILE','WORLD_OBJECT_PRIMARYTYPES','FLAG_BOOL_FIELDS','FLAG_SUBMESSAGE_FIELDS','FLAG_REPEATED_SUBMESSAGE'};fn={'resolve_wiki_family_value','varint','protobuf_fields','decode_flags','decode_submessage','zigzag_uint_to_signed'}
selected=[n for n in tree.body if isinstance(n,ast.Assign) and isinstance(n.targets[0],ast.Name) and n.targets[0].id in names or isinstance(n,ast.FunctionDef) and n.name in fn];exec(compile(ast.Module(body=selected,type_ignores=[]),'actual4f-engine-pure-source-functions','exec'),ns)
helper=rawgt('tools/content-census/item_wiki_family_capture.py',T).decode();ht=ast.parse(helper);resolve=next(n for n in ht.body if isinstance(n,ast.FunctionDef) and n.name=='resolve_infobox_fields');hn={'engine_items':types.SimpleNamespace(resolve_wiki_family_value=ns['resolve_wiki_family_value'])};exec(compile(ast.Module(body=[resolve],type_ignores=[]),'accepted-exact-family-resolver','exec'),hn)
bindings=gt('imports/crystalserver/bindings/items.json')['bindings'];bytarget=collections.defaultdict(list);byexternal=collections.defaultdict(list)
for b in bindings:bytarget[b['target']['key']].append(b);byexternal[b['external_id']].append(b)
clientsha='2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2';clientraw=rawgt(f'content/assets/files/appearances-{clientsha}.dat');assert sha(clientraw)==clientsha
objects={};nameids=collections.defaultdict(list)
for tag,raw in ns['protobuf_fields'](clientraw):
 if tag!=1:continue
 parts=list(ns['protobuf_fields'](raw));o={'id':next(v for k,v in parts if k==1),'name':next((v.decode('utf-8','strict') for k,v in parts if k==4),None),'flags':ns['decode_flags'](next((v for k,v in parts if k==3),b'')),'raw_object_sha256':sha(raw)};assert o['id'] not in objects;objects[o['id']]=o
 if o['name']:nameids[norm(o['name'])].append(o['id'])
refrows=gt('content/world/definitions/reference.json')['records'];native={r['identity']['key']:r for r in refrows if r['kind']=='Item'};assert len(native)==34031
owners=gt('content/world/definitions/declarations.json')['item_authoring'];ownerbykey=collections.defaultdict(list)
for o in owners:ownerbykey[o['item']['key']].append(o)
index=rd(A/'global-infobox-object-own-itemid-index.json');manifest=rd(A/'global-infobox-object-source-manifest.json');assert index['status']==manifest['status']=='COMPLETE_CUTOFF_OWN_ID_INDEX';selection={x['page_id']:x for x in manifest['selected_page_references']};cache={};pages={}
def page(pid):
 if pid in pages:return pages[pid]
 ref=selection[pid];path=Path(ref['capture_path'])
 if path not in cache:
  cache[path]=rd(path);assert w[-1]['sha256']==ref['capture_sha256']
 pg=cache[path]['pages'][ref['page_ordinal']];assert (pg['page_id'],pg['revision_id'],pg['content_sha256'])==(pid,ref['revision_id'],ref['content_sha256']);assert pg['revision_timestamp']<=manifest['qualification_cutoff'];pages[pid]=pg;return pg
for r in old['records']:
 for pid in r['matched_public_source_page_ids']:
  if pid in selection:page(pid)
 for x in index['by_own_itemid_integer_mention'].get(str(r['id']),[]):page(x['page_id'])
newpath=A/'stack-forge-maxima-public-20261002/family-nine-browser-cdp-source.json';fresh=rd(newpath);assert sha(fresh['body'].encode())==fresh['body_sha256'];freshpages=[]
for p in json.loads(fresh['body'])['query']['pages']:
 rev=p['revisions'][0];text=rev['slots']['main']['content'];start=text.find('{{Infobox Object|');assert start>=0
 # Existing top-level parser, no regex field guesses.
 sf=next(n for n in ht.body if isinstance(n,ast.FunctionDef) and n.name=='split_template_params');sn={'re':re};exec(compile(ast.Module(body=[sf],type_ignores=[]),'accepted-top-level-template-params','exec'),sn)
 depth=0;end=None;i=start
 while i<len(text)-1:
  if text[i:i+2]=='{{':depth+=1;i+=2
  elif text[i:i+2]=='}}':depth-=1;i+=2;end=i if depth==0 else None
  else:i+=1
  if end:break
 raw=text[start:end];params=collections.defaultdict(list)
 for term in sn['split_template_params'](raw[2:-2])[1:]:
  if '='in term:k,v=term.split('=',1);params[norm(k)].append(v.strip())
 fields={k:v[0] for k,v in params.items()};family=hn['resolve_infobox_fields'](fields);freshpages.append({'page_id':p['pageid'],'title':p['title'],'revision_id':rev['revid'],'revision_timestamp':rev['timestamp'],'revision_sha1':rev['sha1'],'whole_article_sha256':sha(text.encode()),'raw_infobox_sha256':sha(raw.encode()),'raw_infobox':raw,'parameter_values':dict(params),'accepted_family_resolution':list(family),'cutoff_qualified':rev['timestamp']<=manifest['qualification_cutoff'],'capture_path':str(newpath),'body_sha256':fresh['body_sha256'],'current_public_revision_continuity':'Current anonymous read; no future revision continuity assumed'})
sourcepromotions=gt('tools/content-schema/item-authoring/samples/promotion-crystal-ff7ede5.json')['promotions'];aliases=gt('content/items/aliases.json')['entries'];nameproof=gt('docs/agents/evidence/OTV2-20261002-item-name-source-qualification-v1.json');xmlpath=A/'sources/crystal-ff7ede5-items.xml';xmlraw=xmlpath.read_bytes();assert sha(xmlraw)=='c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb';w.append({'path':str(xmlpath),'sha256':sha(xmlraw),'bytes':len(xmlraw)});xml=ET.fromstring(xmlraw)
mayhem=[];out=[]
for oldr in old['records']:
 n=oldr['id'];key=oldr['item_key'];r=native[key];target=r['identity'];bb=bytarget[key];assert len(bb)==1 and bb[0]['disposition']=='EXACT' and bb[0]['target']==target and bb[0]==oldr['current_binding'];b=bb[0];assert len(byexternal[b['external_id']])==1;external=int(b['external_id']);o=objects.get(external);name=r.get('semantics',{}).get('presentation',{}).get('value',{}).get('name',{'state':'UNKNOWN'});refs=index['by_own_itemid_integer_mention'].get(str(external),[]);proofs=[]
 for pid in sorted({x['page_id'] for x in refs}):
  pg=page(pid)
  for bi,box in enumerate(pg['infobox_objects']):
   if not any(external in x['all_integer_mentions'] for x in box['itemid_occurrences']):continue
   params=box['parameter_values'];fields={k:v[0] for k,v in params.items() if v};family=hn['resolve_infobox_fields'](fields);proofs.append({'page_id':pid,'title':pg['title'],'revision_id':pg['revision_id'],'revision_timestamp':pg['revision_timestamp'],'content_sha256':pg['content_sha256'],'raw_infobox_sha256':sha(box['raw'].encode()),'raw_infobox':box['raw'],'box_index':bi,'balanced':box['balanced'],'inside_comment':box['inside_comment'],'strict_own_id':all(x['strict_positive_comma_ids'] is not None for x in box['itemid_occurrences']),'own_id_values':params.get('itemid',[]),'source_part':selection[pid]['capture_path'],'source_part_sha256':selection[pid]['capture_sha256'],'parameter_values':params,'accepted_family_resolution':list(family)})
 freshmatched=[p for p in freshpages if p['parameter_values'].get('itemid')==[str(external)]];reasons=[]
 if proofs:
  if any(not p['strict_own_id'] for p in proofs):reasons.append('OWN_ID_SYNTAX_PRIORITY_HOLD')
  profs={p['accepted_family_resolution'][0] for p in proofs}
  if None in profs:reasons.append('EXPLICIT_UNADMITTED_OR_ABSENT_FAMILY')
  if len(profs-{None})>1:reasons.append('ALL_OWN_ID_PAGE_FAMILY_CONFLICT')
 else:reasons.append('NO_SELECTED_OWN_ID_FRAME')
 if o and name['state']=='KNOWN' and norm(name['value'])!=norm(o.get('name')):reasons.append('CURRENT_KNOWN_NAME_DIFFERS_EXACT_OFFICIAL_OBJECT_NAME')
 literal=name.get('value') if name['state']=='KNOWN' else None
 if literal=='weapon of mayhem' and o and o.get('name'):
  assert r['semantics']['presentation']['state']=='KNOWN'
  imported=[p for p in sourcepromotions if p['field_path']=='presentation.name' and p['source_item_id']==external];assert len(imported)==1 and imported[0]['typed_value']=={'kind':'TEXT','value':literal};assert [a['target'] for a in aliases if a['key']==imported[0]['native_key'] and a['state']=='ALIAS']==[key];elements=[e for e in xml if int(e.attrib.get('id',e.attrib.get('fromid','0')))<=external<=int(e.attrib.get('id',e.attrib.get('toid','0')))];assert len(elements)==1 and elements[0].attrib.get('name')==literal;assert not any(owner.get('presentation')is not None for owner in ownerbykey[key]);assert o['flags'].get('flags.take')is True and not refs
  mayhem.append({'target':target,'binding':b,'source_item_id':external,'official_name':o['name'],'official_object_sha256':o['raw_object_sha256'],'official_flags':o['flags'],'previous_imported_name':literal,'imported_name_proof':imported[0],'alias_target_verified':key,'protected_XML_source':{'path':str(xmlpath),'sha256':sha(xmlraw),'source_element_attributes':elements[0].attrib,'parsed_subtree_not_raw_bytes':ET.tostring(elements[0],encoding='unicode')},'source_own_id_frame':'NO_OWN_ID_TARGET_ENTRY_IN_SELECTED_GLOBAL_CENSUS','exact_name_family_evidence_pages':oldr['matched_public_source_page_ids'],'historical_family_attempts':oldr['closure_lookup_attempts'],'blocked_source_seam':'Existing Name139 witness_params requires nonempty singleton ownWikiID; cannot reuse/pretend base page IDs apply to missing variant','conditional_closure':'Root-accepted independent exact numeric official name +protected importedliteral exception first; then exact accepted family-name join/navigation-only; no sibling scalar values','status':'QUALIFIED_NUMERIC_NAME_SOURCE_PROPOSAL_NOT_NATIVE_PROMOTION','GameOwned_presentation_absent':True})
 state='CONDITIONAL_NAME15_CORRECTION_THEN_FAMILY' if any(x['target']==target for x in mayhem) else 'HELD_SOURCE_CATEGORY_OR_DOMAIN' if proofs else 'HELD_NO_OWN_ID_OR_QUALIFIED_NAME_SOURCE'
 out.append({'id':n,'target':target,'original_partition_reason':status[n],'current_binding':b,'official_object':o,'native_name':name,'native_materializable':r.get('materializable'),'native_stack_class':r.get('stack_class'),'existing_GameOwned_presentation':any(owner.get('presentation')is not None for owner in ownerbykey[key]),'own_id_priority':bool(refs),'own_id_family_proofs':proofs,'fresh_exact_own_id_pages':freshmatched,'historical_name_join':oldr['closure_lookup_attempts'],'matched_name_pages_as_discovery_only':oldr['matched_public_source_page_ids'],'current_disposition':state,'holds':reasons,'current_public_continuity':'UNKNOWN unless independently captured exact current page; immutable pre-cut proof is not future continuity','fully_verified':'NOT_ESTABLISHED'})
assert len(out)==235 and len(mayhem)==15
# Auxiliary BR articles refine categories only after independently proven own-Fandom identity.
profiles=gt('tools/content-schema/item-authoring/profile-catalog.json')['profiles'];navprofiles=collections.defaultdict(list)
for profile in profiles:
 for category in profile['navigation_families']:navprofiles[category].append(profile['profile_id'])
brpath=A/'stack-forge-maxima-public-20261002/family-nine-br-browser-cdp-source.json';br=rd(brpath);assert sha(br['body'].encode())==br['body_sha256'];brpages=[]
for pg in json.loads(br['body'])['query']['pages']:
 if pg.get('missing'):
  brpages.append({'title':pg['title'],'explicit_missing':True,'capture_path':str(brpath),'capture_sha256':sha(brpath.read_bytes()),'body_sha256':br['body_sha256']});continue
 rev=pg['revisions'][0];content=rev['slots']['main']['content'];start=content.find('{{Infobox_Item|');assert start>=0;depth=0;i=start;end=None
 while i<len(content)-1:
  if content[i:i+2]=='{{':depth+=1;i+=2
  elif content[i:i+2]=='}}':depth-=1;i+=2;end=i if depth==0 else None
  else:i+=1
  if end:break
 assert end is not None
 raw=content[start:end];params=collections.defaultdict(list)
 for term in sn['split_template_params'](raw[2:-2])[1:]:
  if '=' in term:k,v=term.split('=',1);params[norm(k)].append(v.strip())
 fields={k:v[0] for k,v in params.items()};family=hn['resolve_infobox_fields'](fields)
 brpages.append({'page_id':pg['pageid'],'title':pg['title'],'revision_id':rev['revid'],'revision_timestamp':rev['timestamp'],'revision_sha1':rev['sha1'],'whole_article_sha256':sha(content.encode()),'raw_infobox_sha256':sha(raw.encode()),'raw_infobox':raw,'parameter_values':dict(params),'accepted_family_resolution':list(family),'profile_catalog_navigation_candidates':navprofiles.get(fields.get('primarytype'),[]),'cutoff_qualified':rev['timestamp']<=manifest['qualification_cutoff'],'has_own_itemid':bool(params.get('itemid')),'identity_authority':'AUXILIARY_EXACT_NAME_CATEGORY_ONLY_NOT_NUMERIC_ID','capture_path':str(brpath),'capture_sha256':sha(brpath.read_bytes()),'body_sha256':br['body_sha256']})
for candidate in mayhem:
 frames=[]
 for pid in candidate['exact_name_family_evidence_pages']:
  pg=page(pid)
  frames.append({'page_id':pid,'title':pg['title'],'revision_id':pg['revision_id'],'revision_timestamp':pg['revision_timestamp'],'content_sha256':pg['content_sha256'],'source_part':selection[pid]['capture_path'],'source_part_sha256':selection[pid]['capture_sha256'],'own_boxes':[{'raw':box['raw'],'raw_sha256':sha(box['raw'].encode()),'parameter_values':box['parameter_values'],'accepted_resolution':list(hn['resolve_infobox_fields']({k:v[0] for k,v in box['parameter_values'].items() if v}))} for box in pg['infobox_objects']]})
 candidate['retained_name_source_page_frames']=frames
 candidate['scope_of_name_frame']='Accepted family metadata join proof only; own IDs of base article remain distinct from target duration variant; no scalar inheritance; legacy name join still must run after separately accepted correction'
worldroot=A/'frozen-world-3ed537c8';worldseal=rd(worldroot/'frozen-export.json');worldpointers=set();worldids=set()
for wp in sorted(list((worldroot/'content/world/terrain').glob('terrain-*.json'))+list((worldroot/'content/world/objects').glob('objects-*.json'))):
 wb=wp.read_bytes();assert sha(wb)==worldseal['files'][str(wp.relative_to(worldroot))];w.append({'path':str(wp),'commit':worldseal['commit'],'sha256':sha(wb),'bytes':len(wb)})
 for row in json.loads(wb)['records']:
  prov=row.get('provenance',{});ptr=prov.get('item_pointer');sid=prov.get('source_item_id')
  if ptr:worldpointers.add((ptr['family'],ptr['key'],ptr['revision']))
  if sid is not None:worldids.add(sid)
bridges=[]
for r in out:
 n=r['id'];r['world_overlay_owner_absent']=(r['target']['family'],r['target']['key'],r['target']['revision']) not in worldpointers and int(r['current_binding']['external_id']) not in worldids
 if n not in {51526,52745,52785,52789}:continue
 assert native[r['target']['key']]['semantics']['presentation']['state']=='KNOWN'
 o=r['official_object'];assert o['id']==int(r['current_binding']['external_id'])==n and o['flags'].get('flags.take') is True;r['official_name_object_ids']=nameids[norm(o['name'])]
 if len(nameids[norm(o['name'])])!=1:
  r['auxiliary_BR_refinement_hold']='OFFICIAL_NAME_NOT_UNIQUE_SHARED_APPEARANCE_VARIANTS';r['holds'].append('AUXILIARY_BR_NAME_IDENTITY_AMBIGUOUS');continue
 assert r['native_name']['state']=='KNOWN' and norm(r['native_name']['value'])==norm(o['name']);assert r['native_materializable'] is False and r['native_stack_class']=='Unknown' and not r['existing_GameOwned_presentation'] and r['world_overlay_owner_absent']
 assert all(not o['flags'].get(k) for k in ['flags.bank','flags.unmove','flags.corpse','flags.player_corpse'])
 own=r['own_id_family_proofs'];assert len(own)==1 and own[0]['strict_own_id'] and own[0]['balanced'] and not own[0]['inside_comment'];pv=own[0]['parameter_values'];assert pv['itemid']==[str(n)] and pv['primarytype']==['Others'] and pv['pickupable']==['yes'] and pv.get('objectclass')==[''] and not any(v for k in ['secondarytype','status'] for v in pv.get(k,[]))
 hits=[p for p in brpages if not p.get('explicit_missing') and norm(p['title'])==norm(o['name']) and p['parameter_values'].get('name')==[p['title']]];assert len(hits)==1;aux=hits[0];expected='material_valuable' if n==51526 else 'quest_item';assert aux['profile_catalog_navigation_candidates']==[expected] and aux['cutoff_qualified'] and not aux['has_own_itemid'];assert len(aux['parameter_values']['primarytype'])==1
 proposal={'target':r['target'],'source_item_id':n,'current_binding':r['current_binding'],'official_object':o,'native_known_name':r['native_name'],'official_name_unique':True,'world_overlay_owner_absent':True,'GameOwned_presentation_absent':True,'native_materializable':False,'native_stack_class':'Unknown','all_selected_own_id_frames':own,'auxiliary_BR_page':aux,'family_profile':expected,'qualification':'CONDITIONAL_CLOSED_AUXILIARY_SOURCE_BRIDGE_PROPOSAL','identity_authority':'EXACT_CURRENT_BINDING_TO_SAME_OFFICIAL_OBJECT_ID_PLUS_OWN_FANDOM_SINGLETON_ID','semantic_scope':'NAVIGATION_ONLY; no ItemDomain admission, no native classification/scalar/actor-effect inheritance','required_implementation':'Existing family resolver returns null for these BR primarytype strings; separate closed BR source-frame bridge to exact existing profile-catalog navigation category required; existing external18 DOM parser does not accept these new raw Infobox_Item frames','current_public_revision_continuity':'UNKNOWN beyond captured pre-cut revision','source_disagreement':'Fandom Others is unresolved coarse category, not a differing admitted family; every ownID Fandom frame retained'};bridges.append(proposal);r['conditional_BR_refinement']=proposal;r['current_disposition']='CONDITIONAL_AUXILIARY_BR_FAMILY3_BRIDGE'
assert len(bridges)==3
report={'schema':'OTERYN_FAMILY235_STOPPED_RETAINED_AND_BOUNDED_PUBLIC_SOURCE_QUALIFICATION/v1','native_baseline':H,'taxonomy_overlay_baseline':T,'world_overlay_baseline':'3ed537c8b7ab5b9ea9bd26f9de976aaa554a1434','scope':'Whole235classification backlog; navigation/source qualification only; not native classification admission/runtime readiness','exact_partition':dict(collections.Counter(r['original_partition_reason'] for r in out)),'current_dispositions':dict(collections.Counter(r['current_disposition'] for r in out)),'records':out,'conditional_Name15_source_proposals':mayhem,'new_public_nine_pages':freshpages,'new_public_BR_nine_pages':brpages,'conditional_BR3_source_proposals':bridges,'family_new_promotions_applied':0,'fully_verified':'NOT_ESTABLISHED','SourceNo_is_not_no_known_source':'No own-ID frame is not proof of absence of public evidence','existing_stat_primary_guard':'Unadmitted/conflicting retained ownprimary remainshold; no genericfallback/hierarchy stripping','source_access':'Local current Gitblobs and retained source parts; HTTPprobeFandom402/BR403; Root-coordinated sole remaining_fields Chrome/CDP captured9publicrawpages; no browser/project/Git/Cargo writes by this worker','input_witnesses':w}
p=A/'family235-source-qualification-after-forge4f-20261002.json';p.write_text(json.dumps(report,indent=2,ensure_ascii=False)+'\n');q=A/'family235-source-qualification-after-forge4f-20261002.md';q.write_text('Źródłowe rozliczenie 235 pozostałych rodzin Item — native Forge4f, taxonomy88ed\n\nCała zamknięta pula235 ma rekordy z target, exact binding, oficjalnym numeric object, stanem native nazwy, pierwszeństwem własnego WikiID, surowymi source frames/rewizjami i bieżącym hold. Nowych promocji:0. FullyVerified:NOT_ESTABLISHED.\n\n| Partycja pierwotna | ID |\n|---|---:|\n| UNADMITTED_PRIMARYTYPE | 59 |\n| NO_RETAINED_WIKI_ITEM_OBSERVATION | 170 |\n| WIKI_FAMILY_DISAGREEMENT | 4 |\n| NO_EXPLICIT_PRIMARYTYPE | 2 |\n\n| Bieżąca dyspozycja | ID |\n|---|---:|\n| HELD_SOURCE_CATEGORY_OR_DOMAIN | 65 |\n| HELD_NO_OWN_ID_OR_QUALIFIED_NAME_SOURCE | 152 |\n| CONDITIONAL_NAME15_CORRECTION_THEN_FAMILY | 15 |\n| CONDITIONAL_AUXILIARY_BR_FAMILY3_BRIDGE | 3 |\n\nNajwiększa czysta warunkowa kohorta:15 imported Known „weapon of mayhem”. Wszystkie mają exact current Crystal binding do tego samego oficjalnego rawObjectID/name; protected XML oraz stary import potwierdzają pochodzenie obecnego literału; brak GameOwned presentation i world overlay owner. Potrzebna jest osobna zamknięta korekta nazwy, a dopiero potem ponowienie accepted name-family navigation join.\n\n| ID | Exact official name |\n|---|---|\n| 23577 | blade of mayhem |\n| 23583 | axe of mayhem |\n| 23589 | mace of mayhem |\n| 23596 | blade of remedy |\n| 23605 | blade of carving |\n| 23609 | blade of carving |\n| 23619 | mace of remedy |\n| 23624 | axe of carving |\n| 23638 | wand of remedy |\n| 23641 | wand of remedy |\n| 23644 | mace of carving |\n| 23656 | wand of carving |\n| 23659 | rod of carving |\n| 23662 | wand of mayhem |\n| 23665 | rod of mayhem |\n\nŻaden z15 nie ma własnego target ID w pełnym pre-cut own-ID index. Name139 witness_params wymaga własnego singleton ID, więc nie można rozszerzyć jego kohorty przez fałszywy witness ani użyć sibling/base page IDs. Nazwy między wariantami duration mogą się powtarzać; exact numeric official binding stanowi authority korekty, a rodzina jest tylko navigation metadata. Scalar values z base stron nie są dziedziczone. Pełne source frames i dawne join attempts są w JSON.\n\nDruga warunkowa kohorta:3 unikalne oficjalne nazwy52745/52785/52789. Każdy ma własny singleton Fandom ID, primary Others, pickupable=yes, obecne Known name zgodne z exact official name, TakeTrue, brak world owner/GameOwned override. BR raw Infobox_Item ma Itens de Quest, które jest istniejącą kategorią navigation profile quest_item. Obecny family resolver sam zwraca dla tej portugalskiej wartości null: potrzebny jest osobny zamknięty auxiliary raw-BR bridge do istniejącego profilu, nie blanket alias Others ani zmiana existing external18 DOM parsera.\n\nPozostałe wyniki nowych18 publicznych zapytań (9Fandom+9BR):\n\n- SpecialBatCoffin51526: BR Produtos de Criaturas; official name jest też na51527. Hold identity/variant przy dotychczasowym unique-official-name guard.\n- DeadFrog34237: source Fandom Corpses i pickupableyes nie wystarcza do dopisania aliasu portable family. BR Lixos i retained Tibiopedia Creatureparts są rozbieżne; zachować konflikt i owning-domain review.\n- PoemScroll5952: świeża własna strona Fandom wskazuje6119; native Known „scroll” jest inne od exact official „poem scroll”. Nie stosować offsetu167 ani sibling scalar/name inheritance. BR name-only Itens de Quest nie rozwiązuje current Known guard samodzielnie.\n- OakWood52634: własny Fandom Others, BR primarytype pusty; nie fabrykować category z quest context.\n- SpicyFrog31964: Fandom primary/objectclass puste, BR explicit missing; notes event context nie jest explicit statusEvent.\n- UselessPieceOfShield26947: Fandom rodzina pusta, BR explicit missing, Known „Journal Shield” inne niż exact official name; wymagany osobny exact-name/source review.\n\nPozostałych65 source/category/domain holds oraz152 bez qualified own-ID/name frame pozostaje otwartymi zadaniami, nie terminalnym SourceNo. W JSON zachowano wszystkie exact official names i historical discovery titles jako queue dalszej kwalifikacji. Same unmove/noTake nie nadają World family; Trophies(Objects), Tools(Objects), Portals, Teleporters i Tombstones nie są aktualnie admitted WORLD_OBJECT_PRIMARYTYPES. Potrzebny niezależny positive fixed-geometry/corpse predicate albo osobna zatwierdzona bounded alias/domain qualification.\n\nWszystkie18 public query outcomes zachowują pełne API body digest, raw box i revision/cutoff; brakujące BR strony są literalnymi odpowiedziami missing. Ciągłość ponad przechwyconą rewizją pozostaje UNKNOWN. Parent kontroluje implementację i publikację. Nie zmieniono repo, Git/ref ani uruchomiono Cargo.\n')
print(json.dumps({'json':str(p),'sha256':sha(p.read_bytes()),'md':str(q),'md_sha256':sha(q.read_bytes()),'counts':report['current_dispositions'],'Name15':15},indent=2))
