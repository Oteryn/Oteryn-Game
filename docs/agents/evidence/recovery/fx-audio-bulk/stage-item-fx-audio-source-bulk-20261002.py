"""External source staging only: no Git, repo writes, imports, native qualification or runtime claims."""
from pathlib import Path
import bisect,collections,hashlib,json,re,sys,xml.etree.ElementTree as ET
from xml.parsers import expat
OUT=Path('/workspace/audit-continuation'); ROOT=Path('/workspace/pr1437-stack-default-completion'); AV=OUT/'weapon-presentation-source-audit'
sys.path.insert(0,str(ROOT/'tools/content-schema/item-authoring'))
import appearance_membership as membership
import lower_elemental_magic_modifier_packet as elemental
import lower_wiki_stack_default_packet as base
from lower_wiki_movable_packet import leaf
sha=lambda b:hashlib.sha256(b).hexdigest()
def pin(p):
 b=p.read_bytes();return {'path':str(p),'sha256':sha(b),'bytes':len(b)}
def read(p):return json.loads(p.read_bytes())
state=read(OUT/'default8-publication.state.json')
assert state['selected_local_candidate']=='1b026969a8b2236737bff537b3eddb3c909781b2'
PARENT=state['selected_local_candidate']
files={}; retained=[]
for r in read(AV/'captures.json'):
 p=AV/r['commit'][:8]/r['path']; assert p.is_file(),str(p)
 assert sha(p.read_bytes())==r['sha256'] and len(p.read_bytes())==r['bytes']; retained.append(r)
 files[(r['commit'],r['path'])]={'revision':r['commit'],'repository':r.get('repo','zimbadev/crystalserver'),'repository_path':r['path'],**pin(p),'source_url':r['url'],'capture_method':r['method']}
canary=OUT/'canary-weapon-av-47df'; cm=read(canary/'capture-manifest.json')
for r in cm['sources']:
 p=canary/r['path']; assert sha(p.read_bytes())==r['sha256'] and len(p.read_bytes())==r['bytes']
 files[(cm['revision'],r['path'])]={'revision':cm['revision'],'repository':'opentibiabr/canary','repository_path':r['path'],**pin(p),'source_url':r['url'],'capture_method':r['method']}
REV=list(dict.fromkeys([r['commit'] for r in retained])); CFF='ff7ede593c69d4c658b382c97443e8155926924a'; CDN='00ce02a57ca5a12e48f32a3476e37471167e4c3f'; CAN=cm['revision'];REV=[CFF,CDN,CAN]
MEDIA={'effect','shoottype','meleeattackeffect'}
xml_all={}; declarations=[]; xml_counts={}
for rev in REV:
 f=files[(rev,'data/items/items.xml')];raw=Path(f['path']).read_bytes(); newline_offsets=[m.start() for m in re.finditer(b'\n',raw)]; parser=expat.ParserCreate(); depth=0;active=None;node_ranges=[]
 def start(name,attrs):
  global depth,active
  depth+=1
  if depth==2 and name=='item':
   st=parser.CurrentByteIndex;en=raw.find(b'>',st)+1;active={'start':st,'open_end':en,'self_closing':raw[st:en].rstrip().endswith(b'/>')}
 def end(name):
  global depth,active
  if depth==2 and name=='item':
   en=active['open_end'] if active['self_closing'] else raw.find(b'>',parser.CurrentByteIndex)+1
   node_ranges.append((active['start'],en));active=None
  depth-=1
 parser.StartElementHandler=start;parser.EndElementHandler=end;parser.Parse(raw,True)
 et_nodes=[n for n in ET.fromstring(raw) if n.tag=='item'];assert len(et_nodes)==len(node_ranges)
 all_rows=collections.defaultdict(list);counts=collections.Counter();field_ids=collections.defaultdict(set)
 for ordinal,(node,(st,en)) in enumerate(zip(et_nodes,node_ranges)):
  literal=raw[st:en];parsed=ET.fromstring(literal);assert dict(parsed.attrib)==dict(node.attrib)
  ids=[int(node.attrib['id'])] if 'id' in node.attrib else list(range(int(node.attrib['fromid']),int(node.attrib['toid'])+1));assert all(i>=0 for i in ids)
  if not ids:counts['reversed_or_empty_source_id_ranges_unexpanded']+=1
  attrs=[]
  def descend(n,path):
   for idx,child in enumerate(n):
    p=path+[idx]
    if child.tag=='attribute':attrs.append({'child_index_path':p,'attributes':dict(child.attrib),'direct_item_attribute':len(p)==1})
    descend(child,p)
  descend(node,[])
  fields=[a for a in attrs if a['attributes'].get('key','').lower() in MEDIA or 'sound' in a['attributes'].get('key','').lower()]
  witness={'revision':rev,'source_item_ids':ids,'element_ordinal':ordinal,'item_xml_attributes':dict(node.attrib),'name':node.attrib.get('name'),'article':node.attrib.get('article'),'plural':node.attrib.get('plural'),'all_own_attributes':attrs,'media_fields':fields,'source_file':f,'raw_xml':literal.decode(),'raw_xml_sha256':sha(literal),'literal_utf8_byte_span':{'start':st,'end_exclusive':en},'literal_line_span':{'start':bisect.bisect_left(newline_offsets,st)+1,'end':bisect.bisect_left(newline_offsets,en)+1},'source_role':'OTS_HYPOTHESIS_ONLY','variant_scope':'explicit single ID' if len(ids)==1 else 'explicit XML inclusive ID range; no sibling inheritance'}
  for iid in ids:all_rows[iid].append(witness)
  if fields:
   declarations.append(witness);counts['media_declarations']+=1;counts['expanded_media_occurrences']+=len(ids)
   for k in {a['attributes']['key'].lower() for a in fields}:
    counts[k+'_declarations']+=1;field_ids[k].update(ids)
  counts['sound_named_anydepth_attribute_occurrences']+=sum('sound' in a['attributes'].get('key','').lower() for a in attrs)
 xml_all[rev]=all_rows;xml_counts[rev]={'all_xml_item_declarations':len(et_nodes),**dict(counts),'per_field_distinct_source_ids':{k:len(v) for k,v in sorted(field_ids.items())},'media_ids':sorted(set.union(*field_ids.values())) if field_ids else []}
# Complete retained script files and sound/media lexical references, preserving comment disposition.
source_texts=[];script_rows=[];contextual_refs=[]
for (rev,path),f in sorted(files.items()):
 if path.endswith('.xml'):continue
 text=Path(f['path']).read_text();lines=text.splitlines();refs=[]
 for ln,line in enumerate(lines,1):
  if re.search(r'sound|SOUND|Sound',line):refs.append({'line':ln,'raw_line':line,'comment_only':line.lstrip().startswith(('//','--','*')),'tokens':sorted(set(re.findall(r'(?:SoundEffect_t::[A-Z0-9_]+|SOUND_EFFECT_TYPE_[A-Z0-9_]+|COMBAT_PARAM_(?:CAST|IMPACT)SOUND)',line)))})
 if refs or path.endswith('.lua') or 'item_parse' in path or path.endswith('tools.cpp') or path.endswith('weapons.cpp'):
  source_texts.append({'source_file':f,'raw_full_source':text,'all_sound_references':refs,'scope':'Complete retained pinned file; not a complete repository-wide sound or execution census'})
 if not path.endswith('.lua'):contextual_refs.extend({'revision':rev,'repository_path':path,'source_sha256':f['sha256'],**r} for r in refs);continue
 ids=[{'receiver':m.group(1),'source_item_id':int(m.group(2)),'line':text.count('\n',0,m.start())+1,'raw_statement':m.group(0)} for m in re.finditer(r'\b([A-Za-z_][\w]*)\s*:\s*id\s*\(\s*(\d+)\s*\)',text)]
 non_item_ids=[]
 if not path.startswith('data/scripts/weapons/'):
  non_item_ids=ids;ids=[]
 registers=[{'receiver':m.group(1),'line':text.count('\n',0,m.start())+1,'raw_statement':m.group(0)} for m in re.finditer(r'\b([A-Za-z_][\w]*)\s*:\s*register\s*\([^)]*\)',text)]
 params=[{'receiver':m.group(1),'parameter':m.group(2),'literal_expression':m.group(3).strip(),'line':text.count('\n',0,m.start())+1,'raw_statement':m.group(0)} for m in re.finditer(r'\b([A-Za-z_][\w]*)\s*:\s*setParameter\s*\(\s*(COMBAT_PARAM_[A-Z_]+)\s*,\s*([^\n)]*)\)',text) if m.group(2) in ('COMBAT_PARAM_EFFECT','COMBAT_PARAM_DISTANCEEFFECT','COMBAT_PARAM_CASTSOUND','COMBAT_PARAM_IMPACTSOUND')]
 # Group source regions by each literal Weapon declaration; repeated id calls remain distinct.
 starts=[m.start() for m in re.finditer(r'(?:local\s+)?[A-Za-z_]\w*\s*=\s*Weapon\s*\(',text)]
 segments=[]
 if len(starts)>1:
  for i,st in enumerate(starts):
   en=starts[i+1] if i+1<len(starts) else len(text);lo=text.count('\n',0,st)+1;hi=text.count('\n',0,en)+1
   segments.append({'line_start':lo,'line_end':hi,'literal_ids':[a for a in ids if lo<=a['line']<hi],'source_segment':text[st:en],'resolution':'Source lexical region only; not executed event loading'})
 script_rows.append({'source_file':f,'raw_full_source':text,'literal_item_id_calls':ids,'literal_non_item_id_calls':non_item_ids,'non_item_identity_domain':'SPELL_ID_NOT_ITEM_ID' if non_item_ids else None,'register_calls':registers,'media_parameter_calls':params,'media_parameter_scope':'Complete sourcefile calls, not all assigned to every literal ID; use lexical regions/full source for repeated variable declarations','script_domain':'WEAPON_SCRIPT' if path.startswith('data/scripts/weapons/') else 'SPELL_CONTEXT','all_sound_references':refs,'multiple_weapon_regions':segments,'effective_registration':'UNKNOWN_NOT_EXECUTED','sound_precedence':'Outer engine dispatch and callback Combat are separate observations; no flattening/override assertion'})
# Actual frozen current1b witness: no key-suffix inference, no new bindings.
ref_path=ROOT/'content/world/definitions/reference.json';ref=read(ref_path);assert len(ref['records'])==57320
worker=read(OUT/'default8-final-worker-manifest.json')
# Current frozen native hash is verified against already-validated worker manifest.
owned=worker['owned_files']; ownedlist=owned if isinstance(owned,list) else [dict(v,path=k) for k,v in owned.items()]
expected=next(r for r in ownedlist if r['path']=='content/world/definitions/reference.json' or r['path']==str(ref_path));assert pin(ref_path)['sha256']==expected['sha256']
native={r['identity']['key']:r for r in ref['records'] if r['kind']=='Item'};assert len(native)==34031
bd=read(ROOT/base.BINDINGS);forward=collections.defaultdict(list);reverse=collections.defaultdict(list)
for b in bd['bindings']:
 if b.get('identity_namespace')=='ots/item_server_id' and b.get('source_key')=='oteryn:source.crystalserver' and b.get('external_id','').isdigit():forward[int(b['external_id'])].append(b);reverse[b['target']['key']].append(b)
_,admitted=membership.load_admitted(ROOT/'imports/official/appearance-membership');members={label:{r[0]:r for r in m['entries']} for label,m in admitted.items()}
raw=(ROOT/base.CLIENT).read_bytes();assert sha(raw)==base.CLIENT_SHA;objects={}
for tag,body in elemental.protobuf_fields(raw):
 if tag==1:
  obj=elemental.decode_appearance_object(body);assert obj['id'] not in objects;objects[obj['id']]=obj|{'object_sha256':sha(body)}
assert len(objects)==43516
routed={};world_pins=[]
for family in ('terrain','objects'):
 for p in sorted((ROOT/f'content/world/{family}').glob('*.json')):
  world_pins.append(pin(p))
  for row in read(p).get('records',[]):
   target=row.get('provenance',{}).get('item_pointer')
   if target:routed[target['key']]={'world_record_identity':row.get('identity'), 'world_family':family,'item_pointer':target}
decl=read(ROOT/'content/world/definitions/declarations.json');owners={r['item']['key']:r for r in decl['item_authoring']}
xmlids=set().union(*(set(c['media_ids']) for c in xml_counts.values()));scriptids={i['source_item_id'] for s in script_rows for i in s['literal_item_id_calls']};allids=xmlids|scriptids
LABELS={CFF:'crystal-ff7ede5',CDN:'crystal-00ce02a5'}
# Source epoch labels must come from actual admitted labels, not assumed spelling.
import lower_mantra_bond_modifier_packet as mantra
LABELS=mantra.SOURCE_LABELS
rows=[];buckets=collections.defaultdict(list)
for iid in sorted(allids):
 bs=forward.get(iid,[]);b=bs[0] if len(bs)==1 else None; d=native.get(b['target']['key']) if b else None;key=d['identity']['key'] if d else None;obj=objects.get(iid);projection=membership.sha256_hex(membership.canonical_bytes(membership.identity_projection(obj))) if obj else None
 cmem=members['client-15.30'].get(iid);smem=members.get(LABELS.get(b.get('source_revision')) if b else None,{}).get(iid)
 native_name=leaf(d,'presentation.name') if d else {'state':'UNKNOWN'}
 src_byrev={rev:[{'element_ordinal':n['element_ordinal'],'name':n['name'],'item_xml_attributes':n['item_xml_attributes'],'media_fields':n['media_fields'],'raw_xml_sha256':n['raw_xml_sha256']} for n in xml_all[rev].get(iid,[])] for rev in REV}
 direct_byrev={rev:collections.defaultdict(list) for rev in REV}
 for rev in REV:
  for n in xml_all[rev].get(iid,[]):
   for a in n['media_fields']:
    if a['direct_item_attribute']:direct_byrev[rev][a['attributes']['key'].lower()].append(a['attributes'].get('value'))
 opposed=[];absences=[]
 for a,bv in ((CFF,CDN),(CFF,CAN),(CDN,CAN)):
  for field in MEDIA:
   va=direct_byrev[a].get(field,[]);vb=direct_byrev[bv].get(field,[])
   if va and vb and va!=vb:opposed.append({'field':field,'left_revision':a,'left_values':va,'right_revision':bv,'right_values':vb,'role':'OTS_HYPOTHESIS_ONLY_LITERAL_DISAGREEMENT'})
   elif bool(va)!=bool(vb):absences.append({'field':field,'left_revision':a,'left_values':va,'right_revision':bv,'right_values':vb,'role':'SOURCE_PRESENCE_DIFFERENCE_NOT_CONTRADICTION_OR_SILENCE'})
 guards={'SINGLE_FORWARD_SOURCE_BINDING':len(bs)==1,'EXACT_FULL_TARGET_CURRENT_ITEM':bool(b and d and b['disposition']=='EXACT' and b['target']==d['identity']),'SINGLE_REVERSE_BINDING':bool(key and len(reverse[key])==1),'CURRENT_OFFICIAL_OBJECT_PRESENT':bool(obj),'CURRENT_OFFICIAL_FULL_RECORD_MEMBER':bool(obj and cmem and cmem[1]==projection and cmem[2]==obj['object_sha256']),'BOUND_SOURCE_EPOCH_IDENTITY_CONTINUITY':bool(cmem and smem and cmem[1]==smem[1]==projection),'BOUND_OWN_XML_ONE_DECLARATION':bool(b and len(xml_all.get(b['source_revision'],{}).get(iid,[]))==1),'CURRENT_NATIVE_NAME_KNOWN':native_name.get('state')=='KNOWN'}
 actualbound=xml_all.get(b['source_revision'],{}).get(iid,[]) if b else []
 guards['NATIVE_NAME_MATCHES_BOUND_OWN_XML']=bool(native_name.get('state')=='KNOWN' and len(actualbound)==1 and isinstance(actualbound[0]['name'],str) and native_name['value'].strip().casefold()==actualbound[0]['name'].strip().casefold())
 flags=obj.get('flags',{}) if obj else {};world=routed.get(key);bucket='CURRENT_WORLD_OWNER' if world else 'CURRENT_BOUND_ITEM_STATIC_JOIN' if all(guards[k] for k in ('EXACT_FULL_TARGET_CURRENT_ITEM','SINGLE_REVERSE_BINDING','CURRENT_OFFICIAL_FULL_RECORD_MEMBER','BOUND_SOURCE_EPOCH_IDENTITY_CONTINUITY','BOUND_OWN_XML_ONE_DECLARATION')) else 'IDENTITY_OR_MEMBERSHIP_HOLD';buckets[bucket].append(iid)
 scriptref=[{'revision':s['source_file']['revision'],'repository_path':s['source_file']['repository_path'],'source_sha256':s['source_file']['sha256'],'literal_id_calls':[x for x in s['literal_item_id_calls'] if x['source_item_id']==iid],'media_parameter_calls':s['media_parameter_calls'],'multiple_weapon_regions':s['multiple_weapon_regions'],'media_parameter_scope':s['media_parameter_scope'],'effective_registration':s['effective_registration']} for s in script_rows if any(x['source_item_id']==iid for x in s['literal_item_id_calls'])]
 rows.append({'source_item_id':iid,'source_role':'OTS_HYPOTHESIS_ONLY','xml_media_observed':iid in xmlids,'script_literal_id_observed':iid in scriptids,'source_records_by_revision':src_byrev,'existing_binding_candidates':bs,'current_native_identity':d['identity'] if d else None,'current_native_headers':{k:d[k] for k in ('kind','stack_class','client_projection','materializable')} if d else None,'current_native_name':native_name,'current_official_name':obj.get('name') if obj else None,'current_official_flags':flags,'current_official_object_sha256':obj.get('object_sha256') if obj else None,'current_identity_projection_sha256':projection,'current_membership_entry':cmem,'bound_source_membership_entry':smem,'current_world_owner':world,'current_source_item_owner':owners.get(key),'guard_results':guards,'static_identity_applicability_partition':bucket,'explicit_cross_snapshot_media_disagreement':opposed,'cross_snapshot_media_presence_difference':absences,'script_observations':scriptref,'asset_resolution':'UNKNOWN_NO_ASSET_KEYS_INVENTED','native_qualification':'NOT_ATTEMPTED_OTS_HYPOTHESIS_ONLY','runtime_activation':'NONE'})
# A raw observation is evidence, not an accepted new typed Item field.
contract_paths=['docs/architecture/OTERYN_WORLD_PROJECT_SOURCE_PROFILE_V2_DECISION.md','docs/architecture/OTERYN_ITEM_AUTHORING_MASTER_SCHEMA_V1.md','apps/game-server/src/content/project.rs','apps/game-server/src/content/project/v2.rs','apps/game-server/src/content/project/v2/creature.rs','apps/game-server/src/content/reference_playable.rs','apps/game-server/src/content/cw2_b1_import.rs','tools/content-schema/item-authoring/engine_items.py','tools/content-schema/item-authoring/item.schema.json','docs/architecture/reviews/OTERYN_GAME_SPELL_PRESENT0_SPELL_AND_COMBAT_PRESENTATION_DECISION_2026-09-30.md','docs/architecture/OTERYN_GRAPHICS_PRESENTATION_VFX_ARCHITECTURE_BASELINE_2026-09-09.md']
counts={'unique_numeric_source_ids':len(allids),'unique_xml_media_ids_across_snapshots':len(xmlids),'unique_script_literal_ids_across_snapshots':len(scriptids),'xml_script_overlap':len(xmlids&scriptids),'xml_media_declarations_all_three_snapshots':len(declarations),'script_files_per_revision':dict(collections.Counter(s['source_file']['revision'] for s in script_rows)),'weapon_script_files_per_revision':dict(collections.Counter(s['source_file']['revision'] for s in script_rows if s['script_domain']=='WEAPON_SCRIPT')),'spell_context_script_files_per_revision':dict(collections.Counter(s['source_file']['revision'] for s in script_rows if s['script_domain']=='SPELL_CONTEXT')),'script_literal_id_call_occurrences_per_revision':dict(collections.Counter(s['source_file']['revision'] for s in script_rows for _ in s['literal_item_id_calls'])),'script_media_parameter_call_occurrences_per_revision':dict(collections.Counter(s['source_file']['revision'] for s in script_rows for _ in s['media_parameter_calls'])),'source_script_sound_reference_occurrences':sum(len(s['all_sound_references']) for s in script_rows),'weapon_script_sound_reference_occurrences':sum(len(s['all_sound_references']) for s in script_rows if s['script_domain']=='WEAPON_SCRIPT'),'spell_context_script_sound_reference_occurrences':sum(len(s['all_sound_references']) for s in script_rows if s['script_domain']=='SPELL_CONTEXT'),'contextual_cpp_sound_reference_lines':len(contextual_refs),'static_identity_applicability_partition':{k:len(v) for k,v in buckets.items()},'items_with_explicit_cross_snapshot_media_disagreement':sum(bool(r['explicit_cross_snapshot_media_disagreement']) for r in rows),'items_with_source_presence_differences':sum(bool(r['cross_snapshot_media_presence_difference']) for r in rows),'native_known_name_source_disagreement_ids':[r['source_item_id'] for r in rows if r['guard_results']['CURRENT_NATIVE_NAME_KNOWN'] and not r['guard_results']['NATIVE_NAME_MATCHES_BOUND_OWN_XML']],'native_unknown_name_ids':[r['source_item_id'] for r in rows if r['current_native_identity'] and not r['guard_results']['CURRENT_NATIVE_NAME_KNOWN']],'xml_collision_items_by_revision':{rev:sum(len(xml_all[rev].get(iid,[]))>1 for iid in allids) for rev in REV},'current_source_authoring_owners':len(owners),'new_native_fields_or_source_qualifications':0,'new_asset_keys':0}
report={'schema':'OTERYN_ITEM_FX_AUDIO_BULK_RAW_SOURCE_STAGING/v1','status':'FROZEN_OTS_HYPOTHESIS_ONLY_NOT_QUALIFIED_OR_APPLIED','current_parent':PARENT,'historical_4f_audit':pin(OUT/'item-weapon-effects-audio-oteryn-4f-audit-20261002.json'),'access_method':'Local readback of immutable previously retained anonymous public GitHub normal HTTP source bytes and current frozen1b artifacts; no new browser/HTTP/repo/Cargo/Git mutation.','source_role':'OTS_HYPOTHESIS_ONLY; observed implementation/source literals do not establish official Tibia truth, Oteryn assets, accepted runtime rules or Native eligibility.','counts':counts,'overlap_lists':{'xml_and_script':sorted(xmlids&scriptids),'xml_only':sorted(xmlids-scriptids),'script_only':sorted(scriptids-xmlids),'source_revision_xml_media_pairwise':{a[:8]+'__'+b[:8]:sorted(set(xml_counts[a]['media_ids'])&set(xml_counts[b]['media_ids'])) for a,b in ((CFF,CDN),(CFF,CAN),(CDN,CAN))},'identity_partition_ids':dict(buckets),'explicit_media_disagreement_ids':[r['source_item_id'] for r in rows if r['explicit_cross_snapshot_media_disagreement']],'source_presence_difference_ids':[r['source_item_id'] for r in rows if r['cross_snapshot_media_presence_difference']]},'source_files':list(files.values()),'source_manifests':[pin(AV/'captures.json'),pin(canary/'capture-manifest.json')],'xml_snapshot_counts':xml_counts,'xml_media_declarations':declarations,'per_numeric_source_item_rows':rows,'weapon_script_observations':script_rows,'full_selected_context_sources':source_texts,'contextual_cpp_sound_references':contextual_refs,'current_input_pins':[pin(ref_path),pin(ROOT/base.BINDINGS),pin(ROOT/base.CLIENT),pin(ROOT/'content/world/definitions/declarations.json')]+[pin(p) for p in sorted((ROOT/'imports/official/appearance-membership').glob('*.json'))],'world_input_pins':world_pins,'owning_contract_pins':[pin(ROOT/p) for p in contract_paths],'schema_only_candidate':{'status':'REQUIRES_EXPLICIT_OWNING_SOURCE_CONTRACT_IF_TYPED_ITEM_PROJECTION_IS_REQUESTED','current_evidence_route':'Retain exact raw XML/script/code source evidence with repository/revision/fullfileSHA/byte span and explicit source IDs. Existing ImportBatch/ImportCandidate can carry qualified import-candidate evidence only under admitted profile/reimport gates; this external JSON is not that admitted batch.','existing_formal_route':'engine_items.py maps effect/shoottype/meleeattackeffect to formal presentation asset-shaped dependency strings. This mapper route is not proof of admitted assets. Sound mapping absent.','existing_presentation_route':'ProjectV2PresentationAuthoring supports source asset tokens/visual bindings and Cast/Impact/Death/Periodic audio bindings. ItemAuthoring may reference existing Presentation. Reuse only with actual qualified Presentation identity, asset inventory/cues and admitted binding proof; none created here.','optional_source_only_candidate':'If literal media tokens need a typed Item projection before qualified assets, propose optional item_authoring.media_source_observations containing OTS hypothesis source-field/value/source-coordinate observations, scoped to exact existing Item owner and existing import source reference. No AssetKey/event/runtime meaning/default inferred. Contextual rules remain separate source evidence; do not encode C++ or Lua executable callbacks. Owning Source Profile V2 and master/formal source schema revision/strict canonical compatibility must be explicitly amended before implementation.','native_and_runtime':'No Native Item field/profile/codec/protocol additions in this staging. Future combat/event/asset delivery needs its own accepted owner; graphics/SPELL-PRESENT candidate docs grant no production authority.'},'limits':['Full items.xml census for all three pins; no summing snapshots as independent unique Items.','All retained pinned weapon-script files: Crystal10 each/Canary4; complete selected C++ file reference census, not whole-repository sound completeness.','Script literals, repeated id calls and lexical regions are not proof all event variants registered or executed.','Nonempty explicit mismatches are opposition; source absence is not silence, NONE or a contradicted present literal.','XML generic magicEffect often belongs to world/environment objects; not weapon-hit FX by name.','Current static binding/applicability join is not source qualification or admission. Unbound source IDs remain unqualified; no canonical key generated from suffix or borrowed variant.']}
p=OUT/'item-fx-audio-bulk-raw-source-staging-20261002.json';p.write_text(json.dumps(report,ensure_ascii=False,sort_keys=True,indent=2)+'\n')
md=f'''# Efekty/audio Item — zbiorczy staging źródłowy\n\nStatus: OTS_HYPOTHESIS_ONLY, bez kwalifikacji Native, assetów i aktywacji runtime. Aktualny świadek: `{PARENT}`; historyczny audyt4f pozostaje niezmieniony.\n\nPełne trzy XML: {len(declarations)} deklaracji z polami effect/shootType/meleeAttackEffect, {len(xmlids)} unikalnych jawnych ID po rozwinięciu zakresów. Pełne zachowane skrypty broni: Crystal10 na pin i Canary4; {len(scriptids)} ID z literalnych wywołań :id, {len(xmlids&scriptids)} wspólnych z XML. Łączny zamknięty zakres {len(allids)} ID. Wersji nie sumujemy jako nowych Itemów. Każda deklaracja ma literalne pełne XML, SHA, zakres bajtów, nazwę i wszystkie własne atrybuty; każdy skrypt i właściwy zachowany plik C++ ma pełny tekst i pin.\n\nPodział aktualnej tożsamości: {dict((k,len(v)) for k,v in buckets.items())}. Są to wyniki jawnych bindingów/current membership, nie promocje. {counts['items_with_explicit_cross_snapshot_media_disagreement']} ID ma jawny niezgodny token między pinami; {counts['items_with_source_presence_differences']} ma różnicę obecności. Samej nieobecności nie nazywamy ciszą ani sprzecznością.\n\nDźwięków nie zadeklarowano w tych XML. Skrypty broni mają {counts['weapon_script_sound_reference_occurrences']} wierszy odwołań, a jeden dodatkowy skrypt kontekstowego zaklęcia {counts['spell_context_script_sound_reference_occurrences']} wiersze odwołań do dźwięków; wybrane pełne pliki C++ mają {len(contextual_refs)} takich wierszy. To kompletny wykaz zachowanych plików, nie wszystkich źródeł repozytorium. Dźwięk Cast/Impact w skrypcie i zewnętrzny dźwięk ataku/trafienia silnika pozostają oddzielnymi warstwami; literalne ID nie dowodzi wykonania rejestracji. Kierunek, pudło, typ amunicji/broni, żywioł i wynik walki są kontekstem. Nie dopisano wymyślonych eventów, AssetKeys ani numerów sound.\n\nObecny SourceImport/import-candidate model może zachować źródłowe hipotezy pod właściwym profilem importu i pinami. Ten raport jest dowodem zewnętrznym, nie przyjętym importem. Formalny mapper ma trzy pola FX i generuje zależności asset-shaped; nie ma mapowania dźwięków. Istniejący Presentation authoring oferuje powiązania visual/audio i Item→Presentation, ale wymaga realnych kwalifikowanych assetów/cues/tożsamości. Bez nich tokenów OTS nie można przedstawiać jako dopuszczonych zasobów.\n\nJeżeli potrzebny jest nowy typowany owner samych literalnych tokenów przed dopuszczeniem assetów, konkretna propozycja to opcjonalne media_source_observations przy istniejącym ItemAuthoring: wyłącznie source field/value/coordinates i OTS hypothesis, bez semantyki eventu/runtime/defaultów. Wymaga jawnego supplementu owning SourceProfileV2/master/formal schema, revision/migration i strict-compatible canonical tests. Kontekstowe C++/Lua pozostają oddzielnym dowodem, nie nowym programem w danych. Nie wprowadzać Native profile/enum/protocol. Candidate SPELL-PRESENT/graphics dokumenty nie nadają tej paczce autorytetu.\n\nŹródła odczytano lokalnie z wcześniej zapisanych zwykłych publicznych HTTP GitHuba. Nie wykonano nowych zapytań web/RDC, zmian repo, Cargo ani Git/ref. Wszystkie wyniki mają pełne listy ID i sourcefile digests w towarzyszącym JSON.\n'''
mp=OUT/'item-fx-audio-bulk-raw-source-staging-20261002.md';mp.write_text(md)
m={'schema':'OTERYN_ITEM_FX_AUDIO_EXTERNAL_SOURCE_CHECKPOINT/v1','status':report['status'],'baseline':PARENT,'owned_files':[pin(p),pin(mp),pin(Path(__file__))],'counts':counts,'checks':{'all_retained_source_file_bytes':'PASS','xml_literal_spans_reparsed':'PASS','full_current1b_native_worker_pin':'PASS','current_official43516_full_object_decode':'PASS','existing_full_binding_current_membership_static_join':'PASS','Native_or_asset_qualification':'NOT_ATTEMPTED','Cargo_production_tests':'NOT_RUN'}}
manifest=OUT/'item-fx-audio-bulk-raw-source-staging-checkpoint-manifest.json';manifest.write_text(json.dumps(m,sort_keys=True,ensure_ascii=False,indent=2)+'\n');print(json.dumps({'manifest':pin(manifest),'counts':counts,'owned_files':m['owned_files']},indent=2))
