"""External successor; preserves historical Source13 and pending17 bytes."""
import collections
import copy
import hashlib
import json
import pathlib

OUT=pathlib.Path('/workspace/audit-continuation')
sha=lambda raw:hashlib.sha256(raw).hexdigest()
def checked(path,digest=None):
 p=pathlib.Path(path);raw=p.read_bytes()
 if digest:assert sha(raw)==digest,(str(path),sha(raw),digest)
 return json.loads(raw)
def receipt(path):
 p=pathlib.Path(path);raw=p.read_bytes();return {'path':str(p),'sha256':sha(raw),'bytes':len(raw)}
source13_path=OUT/'numeric-modifier13-whole-vector-source-proof-20261002-v2.json'
source13=checked(source13_path,'11b8a6a7f1f85765f2b9c99dabc7815405c2bf3b0c10bb78b39deebb741937c9')
old17_path=OUT/'modifier64-existing-carrier-numeric-source-proposal-20261002.json'
old17=checked(old17_path,'b62b0668bf37982cf4138bdb3a3b051e66352883bfcb4303203e4b04db4985cf')
parent_path=OUT/'equipment19-modifier103-duration54-source-requalification-20261002.json'
parent=checked(parent_path,'815f293f42bc81885db8b45c1cfb8dbe8585c94ca225d6a49349731ce9b6dc19')
peer_path=OUT/'magic-capacity4-independent-observed-parameter-source-peer.json'
peer=checked(peer_path,'6652b18f9a9a410ff9f11de607e7e2fffd10bb3701e3ed7d9b83b588974a7035')
assert not peer['findings'] and peer['native_applied']==0
ids={36672,36673,45639,45640};assert not ids&set(source13['closed_ids'])
# Reuse actual independently verified typednative mapping, explicitly distinguish source labels and native ABI.
kind_keys={'MAGIC_SHIELD_CAPACITY_FLAT':'magicshieldcapacityflat','MAGIC_SHIELD_CAPACITY_PERCENT':'magicshieldcapacitypercent','MAGIC_LEVEL_POINTS':'magiclevelpoints','DEATH_MAGIC_LEVEL_POINTS':'deathmagiclevelpoints','EARTH_MAGIC_LEVEL_POINTS':'earthmagiclevelpoints','FIRE_MAGIC_LEVEL_POINTS':'firemagiclevelpoints','HEALING_MAGIC_LEVEL_POINTS':'healingmagiclevelpoints'}
records=[]
for old in old17['typed_leads_all_including_held']:
 iid=old['item_id']
 if iid not in ids:continue
 identity=parent['identities_by_item_id'][str(iid)];p=next(r for r in peer['records'] if r['item_id']==iid)
 assert not old['blockers'] and all(identity['guard_results'].values())
 assert old['source_whole_vector']==p['whole_vector'] and old['current_target']==p['target']==identity['native_identity']
 assert old['atom_count']==4 and all(a[k]=={'state':'UNKNOWN'} for a in old['source_whole_vector']['value'] for k in ('target_domain','evaluation_phase','priority'))
 checks=[]
 for label,xmlrows in [('binding-selected-Crystal',identity['binding_selected_xml']['records']),('Canary-hypothesis',identity['canary_hypothesis_only'])]:
  assert len(xmlrows)==1
  attrs=collections.defaultdict(list)
  for a in xmlrows[0]['own_attributes']:attrs[a['key'].casefold()].append(a.get('value'))
  atoms=[]
  for atom in old['source_whole_vector']['value']:
   value=atom['parameter']['value'];tag=value['kind'];typed=value['value'];key=kind_keys[atom['kind']]
   if tag=='SIGNED_POINTS':assert type(typed)is int and -(2**31)<=typed<2**31;expected=str(typed)
   elif tag=='RATIONAL_PERCENT':assert type(typed['numerator'])is int and typed['denominator']==1 and 0<=typed['numerator']<=100;expected=str(typed['numerator'])
   else:raise AssertionError(tag)
   assert attrs[key]==[expected]
   atoms.append({'native_kind':atom['kind'],'xml_key':key,'own_literal':attrs[key][0],'parameter_tag':tag,'typed_source_value':typed,'equal':True})
  checks.append({'source':label,'full_own_record_projection_sha256':xmlrows[0]['projection_sha256'],'all_vector_atoms_checked':atoms,'all_agree':True,'scope':'ALL_PROPOSED_MODIFIER_ATOMS; unrelated XML attributes may differ. XML implementation only corroborates public ownWiki observed parameters.'})
 boxes=parent['complete_selected_own_source_boxes_by_item_id'][str(iid)]
 assert len(boxes)==len(p['raw_own_source_witnesses'])==1
 assert boxes[0]['reference']==p['raw_own_source_witnesses'][0]['source_reference']
 assert boxes[0]['raw_sha256']==p['raw_own_source_witnesses'][0]['actual_raw_sha256']
 own_literal=boxes[0]['parameter_values']['attrib'][0]
 assert own_literal in p['raw_own_source_witnesses'][0]['own_attrib_literals']
 record=copy.deepcopy(old);record['qualification']='SOURCE_QUALIFIED_OBSERVED_FLAT_AND_PERCENT_INTRINSIC_PARAMETERS';record.update({'native_modifier_leaf_at_replay':{'state':'UNKNOWN'},'current_published_scope_receipt':{'commit':source13['current_parent']['commit'],'same_complete_native_record_as_verified_b0f':True,'identity':identity['native_identity'],'native_headers':identity['native_headers'],'presentation_name':identity['native_name'],'current_guard_policy':'Scoped identity/name/four headers/source/member/noWorld/Unknown-or-exactKnown complete vector only; parentfullNativehash historicalwitness, notlive siblingguard.'},'official_source':{k:identity[k] for k in ('binding','official_name','official_flags','official_raw_object_sha256','official_identity_projection_sha256','current_member','bound_old_member','bound_old_member_label')},'own_source_boxes':boxes,'all_atom_own_xml_corroboration':checks,'mechanics_documentation_title':'Own Magic Shield Capacity observed literal; native separate parameter types; pinned parser+look corroboration','observed_parameter_only_witness':p['direct_parameter_observation'],'source_meaning':'OwnWiki primary literal +Flat andPercent%, identical named ownXML values and pinnedlook renderroute establish the separate observed numbers; no gear effectivecapacity arithmetic/order inferred.'})
 records.append(record)
assert len(records)==4 and sum(r['atom_count'] for r in records)==16
cpp=[]
for item in peer['cpp_direct_parameter_meaning']:
 path=pathlib.Path(item['path']);raw=path.read_bytes();assert sha(raw)==item['sha256'];lines=raw.decode().splitlines(keepends=True);a,b=item['lines'];quote=''.join(lines[a-1:b]);cpp.append(item|{'exact_quoted_lines':quote,'quote_utf8_offset':sum(len(l.encode()) for l in lines[:a-1]),'whole_byte_hash_verified':True})
lookpeer=peer['Wiki_Look_parameter_route'];lookraw=pathlib.Path(lookpeer['capture']['path']).read_bytes();assert sha(lookraw)==lookpeer['capture']['sha256'];lookapi=json.loads(json.loads(lookraw)['result']['result']['value']['body']);lookpage=next(p for p in lookapi['query']['pages'] if p.get('pageid')==67603);lookrev=lookpage['revisions'][0];assert lookrev['revid']==1131388;lookbody=lookrev['slots']['main']['content'];assert '|{{{attrib|}}}' in lookbody
magic4={'schema':'MAGIC_CAPACITY4_OBSERVED_PARAMETER_EXTERNAL_SOURCE_PROOF/v1','status':'SOURCE_QUALIFIED4_WHOLE16_OBSERVED_PARAMETERS_NOT_AUTHORED_OR_APPLIED','current_parent':source13['current_parent'],'source_transport':source13['source_transport'],'qualification_sources':{'historical17_pending_proposal':receipt(old17_path),'historicalfullsource815f':receipt(parent_path),'independent_magic4_parameter_peer':receipt(peer_path),'truthful_transport_receipt':source13['metadata_supersedes']},'closed_ids':sorted(ids),'counts':{'source_qualified_whole_vectors':4,'source_qualified_atoms':16,'observed_magic_capacity_parameters':8,'preserved_existing_magic_level_sibling_parameters':8,'new_native_applied':0},'observed_parameter_witnesses_cpp':cpp,'public_look_witness':lookpeer|{'full_raw_content':lookbody,'raw_content_sha256':sha(lookbody.encode()),'full_content_role':'Public renderer forwards literalattrib, not calculation formula.'},'native37_parameter_witness':peer['Native_existing_carrier'],'records':sorted(records,key=lambda r:r['item_id']),'semantic_scope':peer['semantic_limits'],'root_scope_authoring_acceptance':'Root explicitly accepted separate4 observed-parameter scope after peer6652b18f; this is user-authorized staticdata enrichment. No public runtime/architecture/profile/type/sourceauthority acceptance manufactured.'}
magicpath=OUT/'magic-capacity4-observed-parameter-source-proof-20261002.json';magicpath.write_text(json.dumps(magic4,ensure_ascii=False,indent=2)+'\n')
all_records=sorted(copy.deepcopy(source13['records'])+copy.deepcopy(magic4['records']),key=lambda r:r['item_id'])
assert len(all_records)==len({r['item_id'] for r in all_records})==17
assert sum(r['atom_count'] for r in all_records)==50
combined={'schema':'NUMERIC_MODIFIER17_WHOLE_VECTOR_EXTERNAL_SOURCE_SUCCESSOR/v1','status':'SOURCE_QUALIFIED17_WHOLE50_NOT_AUTHORED_NOT_APPLIED','current_parent':source13['current_parent'],'source_transport':source13['source_transport'],'original_source13_immutable':receipt(source13_path),'separate_magic4_source':receipt(magicpath),'original_pending17_history_immutable':receipt(old17_path),'source13_independent_peer':receipt(OUT/'numeric-modifier13-independent-source-proposal-peer.json'),'magic4_independent_peer':receipt(peer_path),'closed_ids':[r['item_id'] for r in all_records],'counts':{'source_qualified_whole_vectors':17,'source_qualified_atoms':50,'source13_vectors':13,'source13_atoms':34,'magic4_vectors':4,'magic4_atoms':16,'cohort_intersection':0,'new_native_applied':0,'ABI_schema_type_profile_admission_changes':0},'native37_parameter_witness':source13['native37_parameter_witness'],'mechanics_source_witnesses':source13['mechanics_source_witnesses'],'magic4_observed_parameter_witnesses':{'cpp':magic4['observed_parameter_witnesses_cpp'],'public_look':magic4['public_look_witness']},'records':all_records,'production_authoring_seam':source13['production_authoring_seam']|{'closed_scope':'Explicit successor17 count+ID filter; separate local atomic/idempotent setter handles existingNative37 types; oldstats whitelist/sourceproofs/compiler26/49 remain immutable.','counts':'Exactly17 complete vectors50 atoms; no append14regen, Physical8 orpartial emptyvectors.'},'semantic_scope':source13['semantic_scope']+magic4['semantic_scope'],'retained_holds':{'Physical8':source13['retained_holds']['Physical8'],'LifeLeech50147':source13['retained_holds']['LifeLeech50147'],'Regen14':'Separatefullycharacterized14proposal; nonewholequalified now','source_pair_counts':'17sourcequalified modifierfield pairs of original176pairs138items,159residual heldfieldpairs. ZeroNativeapplied, no whole-itemcompleteness claim.'},'proposed_repository_paths':{'source13_history':'docs/agents/evidence/OTV2-20261002-item-numeric-modifier13-source-qualification-v1.json','magic4_source':'docs/agents/evidence/OTV2-20261002-item-magic-capacity4-source-qualification-v1.json','source17_successor':'docs/agents/evidence/OTV2-20261002-item-numeric-modifier17-source-qualification-v1.json','production_packet':'docs/agents/evidence/OTV2-20261002-item-numeric-modifier17-promotion-v1.json'},'future_compiler_guards':['Exact closed17 IDs/targets, no duplicate/outside/missingLateRow; preserved currentfullBinding EXnamespace +admittedsource/currentmembership +rawobject/name +take/noWorld','Complete global ownID refs/sourcepart/fullrawbox availablehash/coordinates/cutoff/reparse strict singletonID/no duplicated or empty presentmodifierfields, all vectors agree','Native currentidentity/knownsourceName/fourheaders/modifierUnknown-or-exactKnown, blockedGroup/leaf oropposedKnown mustreject; future legitimate sibling fields remainallowed','Existing37order/exacttypedparameter,i32/u16/rationalpercent bounds andEnergy onlyexistingcode; no literalPhysical7/Neutral/all-elementfake defaults','All50 atoms wholevectors preserve all contextsUnknown and allsiblings; local setter stagesall thencommitsatomically, idempotent','CurrentfullNativehash historicalwitness only, no broadfutureScopeguard; actualauthoringparent newreceipt separate historicalsourcefacts whencarry','Full allNative/canonical exact17modifierdelta/source+runtime/library/repo/artifact+packetcontext/migration/tree/Ruff/Cargofinal checks onlyafterRootgrant']}
cp=OUT/'numeric-modifier17-whole-vector-source-successor-20261002.json';cp.write_text(json.dumps(combined,ensure_ascii=False,indent=2)+'\n')
md=OUT/'numeric-modifier17-source-successor-20261002.md';md.write_text('''Jawnie połączono dwa rozłączne zestawy: Numeric13/34 oraz MagicCapacity4/16. Wynik:17 całych wektorów i50 parametrów;0 zmian Native. Starsze dowody13/v2 oraz historyczna propozycja17 zachowują identyczne bajty.

Cztery nowe itemy:36672,36673,45639,45640. Zachowano wszystkie16 atomów, w tym8 wcześniejszych magic-level points. Własne Wiki obserwuje Magic Shield Capacity +80 and8% lub+150 and3%. Własny wybrany XML oraz Canary zgadzają się co do każdego atomu. Pinned Crystal parser zapisuje odrębne wartości Flat/Percent, a renderer pokazuje dokładnie +Flat andPercent%. Jest to zgodność obserwowanych parametrów:80/150SignedPoints oraz8/1 lub3/1RationalPercent. Nie ustalono wzoru, kolejności łączenia, aktywacji, celu ani reguł gry. Wszystkie50 target_domain/evaluation_phase/priority pozostają UNKNOWN.

Wykorzystano istniejące Native37 nośniki. Stary statssetter nie przyjmuje nowych rodzajów; Root zatwierdził osobny zamknięty setter17 z atomowością i idempotencją, zachowujący starszą whitelistę, kompilatory i proofy26/49. Obecny parent1b jest świadkiem źródłowej kwalifikacji, a jego cały Native hash nie staje się przyszłym globalnym guardem. Nowy rzeczywisty parent/carry wymaga odrębnego receipt.

Własne źródła Wiki pochodzą z publicznych zapisów Remote Desktop+Chrome/CDP client_assets; niezależne dokumentacje również odczytano publicznym Chrome po blokadzie zwykłego HTTP402. CPP źródła czterech parametrów przechowano z normalnego publicznego HTTP rawGitHub ff7ede5. Hash własnego rawbox i źródłowej części jest weryfikowany; pełny articleSHA pozostaje deklarowaną współrzędną, bez udawania rekonstrukcji całego artykułu.

Pozostałe159 par pól z zakresu176 nadal są wstrzymane;17 źródłowo zakwalifikowanych także oczekuje wdrożenia. Physical8,14regen,50147 oraz puste grupy nie wchodzą do nowego cohortu. Nie jest to liczba w pełni ukończonych itemów. Nie wykonano WT/Cargo/Git/publikacji.
''')
manifest={'schema':'NUMERIC_MODIFIER17_EXTERNAL_SOURCE_SUCCESSOR_CHECKPOINT/v1','status':'FROZEN_SOURCE17_50_READY_FOR_COVERAGE_AUTHORING_NOT_NATIVE_APPLIED','baseline':source13['current_parent']['commit'],'source_vectors':17,'source_atoms':50,'native_applied':0,'original_histories':{'source13v2':receipt(source13_path),'source13v2manifest':receipt(OUT/'numeric-modifier13-source-checkpoint-manifest-v2.json'),'source17pending':receipt(old17_path),'source13v1':receipt(OUT/'numeric-modifier13-whole-vector-source-proof-20261002.json'),'source13v1manifest':receipt(OUT/'numeric-modifier13-source-checkpoint-manifest.json')},'source_peers':{'numeric13':combined['source13_independent_peer'],'magic4':combined['magic4_independent_peer']},'owned_files':{r['path']:{'sha256':r['sha256'],'bytes':r['bytes']} for r in [receipt(magicpath),receipt(cp),receipt(md),receipt(OUT/'build-magic4-numeric17-source-successor.py')]},'checks':{'disjoint13plus4equals17':'PASS','whole34plus16equals50':'PASS','all16MagicownCrystal_Canary_atomcomparisons':'PASS','all13original_records_unchanged_incombined':'PASS','alloriginalproof_histories_unchanged':'PASS','cpp_exactbytes_lineranges_andpublicLook1131388':'PASS','source17_fullproductionimplementationpeer':'NOT_RUN; sourceauthoring nextbyCoverage','Cargo_generation_fullNative_delta':'NOT_RUN; Rootexclusivegrant required'},'proposed_repository_paths':combined['proposed_repository_paths']}
mp=OUT/'numeric-modifier17-source-successor-checkpoint-manifest.json';mp.write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n')
print(json.dumps({'manifest':receipt(mp),'magic4':receipt(magicpath),'combined17':receipt(cp),'human':receipt(md)},indent=2))
