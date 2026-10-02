from pathlib import Path
import hashlib,json
base=Path('/workspace/audit-continuation')
repo=Path('/workspace/pr1437-stack-default-completion')
proposal_path=base/'imbuement-two-partial-source-owner-proposal-b0f-20261002.json'
facts_path=base/'imbuement-two-own-notes-source-qualified-positive-tier-facts.json'
proposal=json.loads(proposal_path.read_text())
def pin(path):
    b=path.read_bytes();return {'path':str(path),'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b)}
assert pin(proposal_path)['sha256']=='d1cb41d99297ccfda6b79f90274f1d3d578eaa2489564cd9ae6920ead4716bfb'
assert pin(facts_path)['sha256']=='38ee23b0733947544695e4d146734916ea679c28c93e69ac75a7d576001953b8'
assert [(r['source_item_id'],r['source_proven_positive_fact']) for r in proposal['records']]==[(28715,{'family':'SKILL_MAGIC_LEVEL','max_tier':2}),(29427,{'family':'SKILL_MAGIC_LEVEL','max_tier':1})]
contracts=[]
for path,lines in [
 ('docs/architecture/OTERYN_WORLD_PROJECT_SOURCE_PROFILE_V2_DECISION.md',[50,55,71,75,77,79,81,83]),
 ('docs/architecture/OTERYN_ITEM_AUTHORING_MASTER_SCHEMA_V1.md',[158,159,165,166,167,168,169,170,172]),
 ('apps/game-server/src/content/project/v2.rs',[1056,1057,1058,1085,1089,1090]),
 ('apps/game-server/src/content/reference_playable.rs',[254,305,310,621,622,623,624,630,631,636,637,638,639])]:
    p=repo/path;text=p.read_text().splitlines();contracts.append({'repository_relative_path':path,**pin(p),'quoted_lines':[{'line':i,'text':text[i-1]} for i in lines]})
md='''# Proposed owning-contract supplement: two partial imbuement observations

Status: PROPOSAL ONLY. No accepted contract, production schema, native data or executable authority is changed by this document. This review examines the retained proposal and actual contract/code; it does not repeat a complete source corpus audit or run production tests.

The current Source Profile V2 decision, line 75, assigns imbuement semantics to executable Reference Item and says “V2 does not duplicate them.” ProjectV2ItemAuthoring rejects unknown fields and currently has no partial imbuement observation. Neither generic notes, a declarative WorldObject observation nor an augment/use/Forge property supplies this missing Item owner. The intrinsic weapon exception at lines 77/83 supplies a compatibility precedent, not authority for this extension.

Proposed explicit addition to the owning decision, adjacent to line 75:

> As a narrow source-only exception, Item authoring may retain optional `imbuement_allowance_observations` with mandatory closure `PARTIAL_POSITIVE_FACTS`. Each fact names an existing Imbuement family and an explicitly observed source ceiling in Basic=1, Intricate=2, Powerful=3 units. These observations express only the listed family’s positive availability and its stated ceiling. They do not claim a complete family whitelist, a complete exclusion list, active imbues, an executable allowance, or availability of unmentioned families. Presence never lowers into ReferenceItemSemantics. Unknown full Native allowed/excluded vectors remain unknown.

Proposed master-schema source path: `/item/source_observations/imbuement_allowance_facts`. Retain executable `/item/imbuement/allowed_family_tiers` and `excluded_families` separately with their existing whole-vector meaning. Native slot count is untouched. This source ceiling is not a Native `ReferenceImbuementTier` or Forge tier; Native Two/Three/Ten remains unchanged, and Dark Whispers does not add Native One.

The proposed optional owner property is `Option<ProjectV2ItemImbuementAllowanceObservations>`, omitted when absent. Its strict nested payload contains only `closure: PARTIAL_POSITIVE_FACTS` and nonempty `facts` of `{family, max_tier}`. Reuse existing ReferenceImbuementFamily symbols/order (SKILL_MAGIC_LEVEL is existing value 18). Require unique family entries, canonical family ordering, at most 20 entries, and integer ceilings 1..3. Reject unknown keys/closures/families, empty or duplicate entries, booleans, floats and out-of-range tiers. No source-ID binding is embedded in the property: its existing ItemAuthoring owner binds the full existing Item target.

A future accepted supplement must explicitly name its source schema revision and migration/canonical compatibility disposition, as required by the decision at line 50. Do not silently publish a new property under an unchanged authoritative schema identity. The intended extended reader accepts old absent-property documents and preserves their canonical bytes; old strict readers reject documents carrying the new property. This is a fail-closed source-reader extension, not bidirectional reader compatibility. Native artifacts, codecs, network protocol, runtime admission and tier enums do not change. The precise revision identifier is for the owning amendment to decide, not assigned by this proposal.

The closed first packet contains exactly two facts: Falcon Coif i28715, SKILL_MAGIC_LEVEL ceiling 2; Dark Whispers i29427, SKILL_MAGIC_LEVEL ceiling 1. Their full own Notes and real browser-captured page/revision/date/content coordinates remain evidence. Dark Whispers’ explicit Intricate/Powerful exclusion stays quoted; it does not become an excluded-family list. XML broad family lists remain corroborating hypotheses, and a missing XML family is absence rather than an inferred denial. No general Notes parser or sibling-page inheritance is introduced.

Before authoring, a fresh current-parent receipt must validate both complete targets, actual membership and binding namespace/source revision in both directions, known source names, own ID priority and all selected own-page opposition, native admission/class headers, two known slots, and no current World owner. Retain b0f full hashes as historical witnesses; do not label them the future current parent or pin unrelated future Native sibling fields as live guards. Compare the actual parent’s source-owner records and merge only the optional observation property into existing owners. The b0f proposal’s 411→413 forecast is not a current census: later Forge owners can make the true additional owner count 0, 1 or 2. Preserve all authoring siblings, bindings, Native definitions, names and imbuement vectors exactly. Prevalidate the entire two-target packet before mutation; reject a late conflict atomically and support an identical idempotent reapplication.

Required owning changes if accepted: Source Profile V2 decision and master-schema path/units; strict optional ProjectV2ItemAuthoring structs and validation/canonicalization; formal source-only schema and generated schema; a dedicated closed two-fact source compiler/packet plus atomic authoring merge; migration/repository inventory checks for the actual two payloads; CI routes and source evidence packaging. Do not change ReferenceImbuementTier, ReferenceItemImbuement, artifact profile, Runtime state, unrelated alias catalogs or Native lowering.

Required checks remain unrun: old absent-field canonical goldens; present-field old-reader rejection and new-reader roundtrip; malformed/unknown/duplicate/empty/out-of-range/type-invalid payload rejection; exact own source/binding/name/member/World and opposite-known guards; atomic late failure and idempotence; existing-authoring-owner and new-owner merge cases; all current Native records and imbuement slot/allowed/excluded values unchanged; independent source and owning-contract acceptance. Two observed facts are not two complete Items and are not two Native eligible whole vectors.
'''
md_path=base/'imbuement2-partial-source-owning-contract-amendment-candidate.md'
md_path.write_text(md)
review={
 'schema':'OTERYN_IMBUEMENT2_INDEPENDENT_SOURCE_CONTRACT_REVIEW/v1',
 'status':'REQUIRES_NARROW_OWNING_SOURCE_CONTRACT_AMENDMENT_NOT_IMPLEMENTED',
 'review_scope':'Actual minimal owning contract and Rust carrier readback plus unchanged retained two-fact proposal; no fresh full-source-corpus replay, production authoring, Cargo or acceptance claim.',
 'reviewed_proposal':pin(proposal_path),'retained_positive_source_facts':pin(facts_path),
 'historical_baseline':proposal['baseline'],
 'counts':{'positive_source_fact_entries':2,'target_items':2,'native_whole_vector_eligible':0,'native_applied':0,'complete_items_claimed':0,'future_additional_authoring_owners_min':0,'future_additional_authoring_owners_max':2},
 'disposition':{'existing_source_owner':'ProjectV2ItemAuthoring','current_typed_carrier':'ABSENT','current_contract_route':'NOT_ADMITTED_BY_LINE_75','evidence_retention':'Existing retained evidence remains valid without typed promotion','required_action':'Explicit narrow owning source-contract supplement before production carrier/compiler writes','native_tier_one':'NOT_PRESENT_AND_NOT_PROPOSED','full_native_whitelist':'UNKNOWN_PRESERVED','full_native_exclusion_list':'UNKNOWN_PRESERVED'},
 'contracts':contracts,
 'accepted_contract_conflicts':[{'path':contracts[0]['repository_relative_path'],'line':75,'reason':'Explicit executable ownership/nonduplication covers imbuement; partial source observation exception is not present.'},{'path':contracts[0]['repository_relative_path'],'line':50,'reason':'A published field/authority change requires named schema revision and migration; existing optional weapon exception is not authority for this new field.'},{'path':contracts[2]['repository_relative_path'],'line':1057,'reason':'Current strict Item authoring rejects the proposed new field.'}],
 'proposed_amendment':pin(md_path),
 'closed_records':[{'source_item_id':r['source_item_id'],'target':r['target'],'positive_source_fact':r['source_proven_positive_fact'],'source':r['own_source'],'current_headers_historical':r['actual_b0f_current_native_headers'],'current_name_historical':r['actual_b0f_current_native_name'],'current_imbuement_historical':r['actual_b0f_current_native_imbuement']} for r in proposal['records']],
 'future_current_parent_guard':'Must be a separate truthful receipt at actual published/carry parent. Historical b0f full native digest is not a live whole-native/sibling guard. Exact own targets, names, class/admission, slots, source facts/bindings/membership/noWorld remain guarded.',
 'source_transport':'Previously retained public Chrome/CDP browser capture; no new website access in this contract review. OTS XML corroboration is hypothesis-only and not new native source authority.',
 'compatibility':{'old_native_bytes':'UNCHANGED_REQUIRED','old_source_absent_field_canonical_bytes':'UNCHANGED_REQUIRED','new_source_field_old_strict_reader':'REJECT_REQUIRED','new_reader_old_source':'ACCEPT_REQUIRED','new_source_schema_revision_and_migration':'OWNER_ACCEPTANCE_REQUIRED','native_tier_enum':'UNCHANGED_TWO_THREE_TEN'},
 'implementation_and_test_status':'NOT_IMPLEMENTED_NOT_RUN',
 'findings':[{'code':'OWNING_SOURCE_ROUTE_MISSING','severity':'BLOCKS_PRODUCTION_AUTHORING_UNTIL_EXPLICIT_SUPPLEMENT','disposition':'Concrete narrow amendment candidate attached; does not negate retained positive source facts'},{'code':'HISTORICAL_OWNER_CENSUS_ONLY','severity':'RECEIPT_CORRECTION_REQUIRED_AT_IMPLEMENTATION','disposition':'411→413 is b0f forecast; derive future delta 0..2 from actual owner merge, never repeat as current'}]
}
review_path=base/'imbuement2-independent-partial-source-contract-review.json'
review_path.write_text(json.dumps(review,ensure_ascii=False,sort_keys=True,indent=2)+'\n')
manifest={'schema':'OTERYN_IMBUEMENT2_EXTERNAL_CONTRACT_REVIEW_CHECKPOINT/v1','status':'FROZEN_PROPOSAL_ONLY_REQUIRES_OWNING_AMENDMENT','historical_baseline':proposal['baseline'],'owned_files':[pin(md_path),pin(review_path),pin(Path(__file__))],'checks':{'proposal_hash':'PASS','positive_source_fact_hash':'PASS','closed_two_fact_identity_and_ceiling':'PASS','minimal_contract_and_carrier_readback':'PASS','production_tests':'NOT_RUN','fresh_current_parent_source_audit':'NOT_RUN','owning_acceptance':'NOT_GRANTED'}}
mp=base/'imbuement2-partial-source-contract-review-checkpoint-manifest.json'
mp.write_text(json.dumps(manifest,ensure_ascii=False,sort_keys=True,indent=2)+'\n')
print(json.dumps({'manifest':pin(mp),'owned_files':manifest['owned_files'],'status':manifest['status']},indent=2))
