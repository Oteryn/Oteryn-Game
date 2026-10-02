"""Source-only14 additive proposal; no values transferred from equip targets."""
import collections
import hashlib
import json
import pathlib

OUT=pathlib.Path('/workspace/audit-continuation');sha=lambda b:hashlib.sha256(b).hexdigest()
p=OUT/'equipment19-modifier103-duration54-source-requalification-20261002.json';source=json.loads(p.read_bytes())
ids={3052,3098,3549,6529,8081,12669,23474,23475,23476,23477,23531,23532,23543,45643}
# Explicit OWN notes witness tuples, not XML-derived facts. No normalization of tick quantum.
wiki_tuple={3052:(2,8,6000),3098:(6,24,6000),3549:(3,12,6000),6529:(3,12,6000),8081:(2,8,6000),23474:(2,8,6000),23475:(2,8,6000),23476:(3,12,6000),23477:(3,12,6000),23531:(1,4,6000),23532:(1,4,6000),23543:(2,8,6000),45643:(None,16,6000)}
# Exact quoted OWN note substrings; wikitext retained, no broad markup or name alias inference.
quotes={3052:'gives 8 [[manapoint]]s and 2 [[hitpoint]]s every 6 seconds',3098:'at a rate of 24 [[Mana Points|mp]] and 6 [[Hit Points|hp]] per 6 seconds',3549:'by the rate of 3 [[HP]] and 12 [[Mana|MP]] per 6 seconds',6529:'by the rate of 3 [[HP]] and 12 [[Mana|MP]] per 6 seconds',8081:'8 manapoints and 2 hitpoints every 6 seconds',23474:'by the rate of 2 [[HP]] and 8 [[Mana|MP]] per 6 seconds',23475:'by the rate of 2 [[HP]] and 8 [[Mana|MP]] per 6 seconds',23476:'by the rate of 3 [[HP]] and 12 [[Mana|MP]] per 6 seconds',23477:'by the rate of 3 [[HP]] and 12 [[Mana|MP]] per 6 seconds',23531:'by the rate of 1 [[HP]] and 4 [[Mana|MP]] per 6 seconds',23532:'by the rate of 1 [[HP]] and 4 [[Mana|MP]] per 6 seconds',23543:'by the rate of 2 [[HP]] and 8 [[Mana|MP]] per 6 seconds',45643:'restores 16 [[Mana]] every 6 seconds'}
records=[]
for iid in sorted(ids):
 identity=source['identities_by_item_id'][str(iid)];boxes=source['complete_selected_own_source_boxes_by_item_id'][str(iid)]
 raw_group=[{k:v for k,v in b['parameter_values'].items() if k in ('attrib','mantra','elementalbond','crithit_ch','critextra_dmg','hpleech_am','hpleech_ch','manaleech_am','manaleech_ch')} for b in boxes]
 assert all(any('faster regeneration' in v.casefold() for v in g['attrib']) for g in raw_group)
 note_witnesses=[]
 for b in boxes:
  notes=b['parameter_values'].get('notes',[]);assert len(notes)==1
  quote=quotes.get(iid)
  if quote:assert notes[0].count(quote)==1
  note_witnesses.append({'reference':b['reference'],'raw_template_sha256':b['raw_sha256'],'own_notes':notes[0],'exact_tuple_quote':quote,'quote_char_offset':notes[0].index(quote) if quote else None,'tuple':{'health_gain_points':wiki_tuple[iid][0],'mana_gain_points':wiki_tuple[iid][1],'ticks_milliseconds':wiki_tuple[iid][2]} if iid in wiki_tuple else None,'seconds_to_milliseconds':'EXACT6s=>6000ms source unit conversion only; no active clock interpretation' if iid in wiki_tuple else None})
 xml_checks=[]
 for label,xmlrows in [('binding-selected-Crystal',identity['binding_selected_xml']['records']),('Canary-hypothesis',identity['canary_hypothesis_only'])]:
  assert len(xmlrows)==1
  attrs=collections.defaultdict(list)
  for a in xmlrows[0]['own_attributes']:attrs[a['key'].casefold()].append(a.get('value'))
  gains={k:attrs.get(k,[]) for k in ('healthgain','healthticks','managain','manaticks')}
  own_tuple={k:int(v[0]) for k,v in gains.items()} if all(len(v)==1 and v[0].isdecimal() for v in gains.values()) else None
  target=attrs.get('transformequipto',[])
  xml_checks.append({'source':label,'source_record_projection_sha256':xmlrows[0]['projection_sha256'],'own_regeneration_attributes':gains,'own_numeric_tuple':own_tuple,'transformequipto_own_literal':target,'all_four_own_values_present':own_tuple is not None,'target_transfer_allowed':False})
 current_tuple=xml_checks[0]['own_numeric_tuple'];canary_tuple=xml_checks[1]['own_numeric_tuple'];same=current_tuple is not None and current_tuple==canary_tuple
 single=identity['guard_results']['ALL_OWN_ID_BOXES_EXACT_SINGLE_ID']
 holds=[]
 if current_tuple is None:holds.append('OWN_BOUND_XML_TUPLE_ABSENT_EQUIP_TARGET_TRANSFER_PROHIBITED')
 if not single:holds.append('SHARED_OWN_ID_PHASE_UNQUALIFIED')
 if iid==12669:holds.append('NO_OWN_WIKI_NUMERIC_REGENERATION_TUPLE')
 if iid==8081:holds.append('WIKI_XML_TICK_QUANTUM_DISAGREEMENT_NO_AVERAGE_RATE_SUBSTITUTION')
 if iid==45643:holds.append('WIKI_XML_MANA_RATE_DISAGREEMENT_AND_PUBLIC_HEALTH_AMOUNT_ABSENT')
 if current_tuple and not same:holds.append('OWN_CRYSTAL_CANARY_REGENERATION_DISAGREEMENT')
 assert holds
 whole_source_other_atoms={'faster regeneration':'Cannot drop this observed effect while retaining other numeric modifiers.', 'attrib':raw_group}
 records.append({'item_id':iid,'item_key':identity['item_key'],'identity_and_current_guards':identity,'own_raw_modifier_groups':raw_group,'own_note_tuple_witnesses':note_witnesses,'all_own_xml_checks':xml_checks,'same_own_Crystal_Canary_tuple':same,'native_carriers_available':{'HEALTH_GAIN':'SIGNED_POINTS/i32','HEALTH_TICKS':'MILLISECONDS/u64','MANA_GAIN':'SIGNED_POINTS/i32','MANA_TICKS':'MILLISECONDS/u64'},'native_context_policy':'UNKNOWN target_domain/evaluation_phase/priority; no ruleset, food clock, rest mode or activation source fabricated','source_guard_holds':holds,'disjoint_primary_reason':holds[0],'classification':'SOURCE_ONLY_TYPED_TUPLE_LEAD_HELD_NOT_NATIVE_ELIGIBLE','remaining_present_group':whole_source_other_atoms})
assert len(records)==14
proof={'schema':'REGENERATION14_EXISTING_NATIVE_CARRIER_SOURCE_PROPOSAL/v1','status':'WHOLE14_EXACT_SOURCE_CHARACTERIZED_ZERO_QUALIFIED_UNDER_CURRENT_OWN_ID_AND_OWN_XML_GUARDS','baseline_source_replay':'b0f91e2883007861f5556c44cce13701bb447283','current_source_receipt_commit':'1b026969a8b2236737bff537b3eddb3c909781b2','prior_numeric13_source_immutable':True,'parent_report_path':str(p),'parent_report_sha256':sha(p.read_bytes()),'source_transport_override':'All complete selected own Wiki frames are anonymous browser same-origin API captures by client_assets, per actual global manifest. Parent ordinaryHTTP prose is superseded by numeric13 transport receiptv2. XML locally replayed retained public source, hypothesis/corroboration only.','counts':{'whole_source_ids':14,'own_Wiki_explicit_rate_tuple':13,'own_Wiki_no_numeric_rate':1,'all_four_own_XML_attributes_present':6,'own_XML_missing_equip_target_only':8,'own_XML_complete_but_shared_own_Wiki_ID':4,'own_XML_complete_single_Wiki_ID':2,'whole_native_eligible':0,'native_applied':0,'disjoint_primary_reasons':dict(collections.Counter(r['disjoint_primary_reason'] for r in records))},'native_boundary':'Native37 has kinds13/14/22/25 and parameterSignedPoints/Milliseconds already. Existing stats apply_packet MODIFIERS whitelist excludes these; any accepted future source tuple needs distinct closed local setter, not enum/schema/protocol/profile expansion.','source_meaning_limits':['Do not replace source2HP/8mana every6s with1HP/4mana every3s on8081 merely because average rates agree; the explicit tick quantum differs. No gameplay clock assumption permitted.','45643 own notes16mana per6s versus boundXML4mana per3s are opposing even as average rates, with public health tuple absent; no alternate value chosen.','Eight absent-own XML tuples cannot be filled by another equipTargetID; public notes prove a named effect, not own bound-source phase equivalence.','Four equipped IDs with numeric own XML remain source-shared with base IDs; source-own-ID phase qualification is not silently inferred from XMLtransform data.','13 ownNote numeric rate witnesses coexist with food/rest/vocation/normalregeneration clauses; retain all clauses. No inference of activation, rest doubling or clock domain.','No partial Known vector by keeping speed/magic level/life leech while omitting fasterregeneration.','Generic documentation needed only to explain parameter units/modifier-vs-total semantics; it cannot assign anotherID phase or erase actual source disagreement.','13numeric34 source-qualified proof remains distinct and unchanged; these14 are add-on research, not an expanded oldcohort.'],'required_bounded_public_doc':['Regeneration: intrinsic item gainpoints and repeat interval units; additive versus normal/vocation/rest rates; no native runtimeclock inference.'],'records':records}
q=OUT/'regeneration14-own-notes-xml-existing-carrier-source-proposal-20261002.json';q.write_text(json.dumps(proof,ensure_ascii=False,indent=2)+'\n');print(json.dumps({'path':str(q),'sha256':sha(q.read_bytes()),'bytes':q.stat().st_size,'counts':proof['counts']},indent=2))
