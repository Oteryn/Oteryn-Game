"""Materialize only reviewed r42/r44 pre-Combat healing candidates, inactive."""
import argparse
import gzip
import json
from pathlib import Path

from import_source_spell_package import ROOT,digest,encoded,require
from source_spell_import_guards import read_base,read_package,write_source_only_set

BASE_SHA='1c8ed40b00c5457ad408cbfd918147e0e1c197cd5877f7f8e8f18305e9cb7b5f'
CONFIGS={
 'r42':{'source':'docs/reference/spells/r42-source-closure/player-heal-friend-candidate','registration_key':'canary-main-current/data/scripts/spells/healing/heal_friend.lua#1','manifest_sha256':'8cf2c43dd4ecc48f308029bab15f5a0aca0836bf6138eebd3febf5335319b031','proof_sha256':'aac9c599f2a507c3a0a46399319d814755ef29f87f8095a56edc9f6a79011fac','proof_schema':'OTERYN_SOURCE_HEAL_FRIEND_QUALIFICATION/v1'},
 'r44':{'source':'docs/reference/spells/r44-source-closure/player-restore-balance-candidate','registration_key':'crystal-summer-current/data/scripts/spells/healing/restore_balance.lua#1','manifest_sha256':'ddfcfa978a3d830e06662c9ceca1abb20dc5370d5931c1fe27980faf516f5458','proof_sha256':'c8d2828e5d07422faea0549e816aa4300cb9e199d5626dd866fb19d9cc81e097','proof_schema':'OTERYN_SOURCE_RESTORE_BALANCE_QUALIFICATION/v1'}}

def canonical(value):return json.dumps(value,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()

def validate_dependency_closure(spell,deps,catalog,candidate_key,revision):
    require(set(deps)=={'abilities','effects','formulas'} and {k:len(v) for k,v in deps.items()}=={'abilities':1,'effects':2,'formulas':1},'DEPENDENCY_POPULATION_MISMATCH')
    definitions={}
    for plural,family in [('abilities','Ability'),('effects','Effect'),('formulas','Formula')]:
        for item in deps[plural]:
            identity=item['identity'];require(identity['revision']==revision and identity['key'].startswith(candidate_key+'/'),'DEPENDENCY_NAMESPACE_MISMATCH')
            ident=(family,identity['key'],identity['revision']);require(ident not in definitions,'DEPENDENCY_DUPLICATE_IDENTITY');definitions[ident]=item
    references=[]
    def visit(obj):
        if isinstance(obj,dict):
            if set(obj)=={'family','key','revision'}:references.append((obj['family'],obj['key'],obj['revision']))
            for value in obj.values():visit(value)
        elif isinstance(obj,list):
            for value in obj:visit(value)
    visit(spell);visit(deps)
    require(all(ref in definitions for ref in references),'DEPENDENCY_REFERENCE_UNRESOLVED')
    require(set(references)==set(definitions),'DEPENDENCY_UNREACHABLE_DEFINITION')
    require(spell['spell']['execution']=={'ability':{'family':'Ability','key':deps['abilities'][0]['identity']['key'],'revision':revision}},'PRECOMBAT_EXECUTION_NOT_ABILITY')
    require(catalog=={'definitions':[]},'EXTERNAL_CATALOG_REFUSED')
    effect=deps['effects'][0]
    require(effect.get('operation')=='heal' and effect.get('damage_type')=='healing' and effect.get('presentation',{}).get('caster_effect_timing')=='before_combat','PRECOMBAT_HEALING_SEMANTICS_MISMATCH')
    require(deps['effects'][1].get('operation')=='remove_condition' and deps['effects'][1].get('removed_condition')=='paralyze','PARALYZE_DISPEL_MISMATCH')

def prepare_import(root,revision,source=None):
    require(revision in CONFIGS,'UNSUPPORTED_PRECOMBAT_FAMILY')
    config=CONFIGS[revision];source=root/config['source'] if source is None else source
    require(source.resolve().is_relative_to(root.resolve()),'SOURCE_OUTSIDE_REPOSITORY')
    base,base_body,baseline,schemas,validators=read_base(root,BASE_SHA)
    packet=read_package(source)
    require(digest(packet['package-manifest.json'])==config['manifest_sha256'],'SOURCE_REVIEW_PIN_MISMATCH')
    require(digest(packet['source-qualification-proof.json'])==config['proof_sha256'],'PROOF_REVIEW_PIN_MISMATCH')
    require(packet['receipt.schema.json']==schemas['schemas/player-bundle-receipt.schema.json'],'RECEIPT_SCHEMA_CHANGED')
    summary=json.loads(packet['import-summary.json']);proof=json.loads(packet['source-qualification-proof.json'])
    reg=config['registration_key'];snapshot,path=reg.split('/',1);path=path.rsplit('#',1)[0]
    source_pin=baseline['source_pins']['player_snapshots'][snapshot]
    candidate_revision='source-player-'+revision;row_id=digest(reg.encode())[:16];folder=snapshot+'/'+row_id
    candidate_key='candidate:spell/source/'+folder
    expected_names={'package-manifest.json','import-summary.json','source-callback-facts.jsonl.gz','source-qualification-proof.json','receipt.schema.json'}|{folder+'/'+name for name in ['source-header.json','receipt.json','spell.json','dependencies.json','catalog.json']}
    require(set(packet)==expected_names,'PRECOMBAT_PACKET_MEMBERSHIP_MISMATCH')
    require(summary['records']==1 and summary['revision']==candidate_revision and summary['status_counts']=={'CANDIDATE_SCHEMA_VALID':1}
            and summary['runtime_activation'] is False and summary['external_sources_used'] is False and summary['native_descriptor_count']==0
            and summary['full_source_mechanics_1_to_1_complete'] is False,'SOURCE_STATUS_MISMATCH')
    require(summary['records_index']==[{'blockers':[],'candidate_key':candidate_key,'candidate_revision':candidate_revision,'registration_key':reg,'status':'CANDIDATE_SCHEMA_VALID'}],'SOURCE_INDEX_MISMATCH')
    require(set(summary['input_proofs'])=={'r28/package-manifest.json','r28/import-summary.json'},'SOURCE_INPUT_PROOFS_MISMATCH')
    for name in ['package-manifest.json','import-summary.json']:
        require(summary['input_proofs']['r28/'+name]==digest((base/'player-source-bundles'/name).read_bytes()),'BASE_PLAYER_INPUT_PIN_MISMATCH')
    require(proof['schema']==config['proof_schema'] and proof['qualification_scope']=='exact_source_wrapper_to_existing_authoring_schema'
            and proof['mechanics_qualification']=='wrapper_schema_qualified_engine_execution_unqualified'
            and all(proof[k] is False for k in ['runtime_activation','native_execution_qualified','native_identity_allocation','canonical_selection_changed','external_sources_used','legacy_matcher_modified']), 'PROOF_QUALIFICATION_MISMATCH')
    payload=gzip.decompress(packet['source-callback-facts.jsonl.gz']);callbacks=[json.loads(line) for line in payload.splitlines()]
    require(digest(payload)==summary['source_callback_payload_sha256'] and len(callbacks)==1 and callbacks[0]['registration_key']==reg,'CALLBACK_POPULATION_OR_PAYLOAD_MISMATCH')
    callback=callbacks[0];base_callback=(base/'player-source-bundles/source-callback-facts.jsonl.gz').read_bytes()
    old={r['registration_key']:r for r in map(json.loads,gzip.decompress(base_callback).splitlines())}[reg]
    require(callback==old and callback['source_revision']==source_pin,'CALLBACK_SOURCE_CHANGED')
    old_folder=base/'player-source-bundles'/folder;old_receipt=(old_folder/'receipt.json').read_bytes();old_header=(old_folder/'source-header.json').read_bytes()
    require(json.loads(old_receipt)['status']=='BLOCKED','BASE_REGISTRATION_NOT_BLOCKED')
    require(packet[folder+'/source-header.json']==old_header,'HEADER_SOURCE_CHANGED')
    require(proof['historical_receipt_sha256']==digest(old_receipt) and proof['historical_header_sha256']==digest(old_header)
            and proof['source_capture_gzip_sha256']==digest(base_callback) and proof['selected_capture_fact_sha256']==digest(canonical(callback)), 'PROOF_HISTORICAL_INPUT_MISMATCH')
    require(proof['registration_key']==reg and proof['source_path']==path and proof['source_revision']==source_pin
            and proof['source_sha256']==callback['source_sha256'] and proof['source_git_blob']==callback['source_callback_facts']['blob']
            and proof['candidate_key']==candidate_key and proof['candidate_revision']==candidate_revision,'PROOF_SOURCE_IDENTITY_MISMATCH')
    require(proof['formula_fact_sha256']==digest(canonical(callback['source_callback_facts']['combats'][0]['callbacks'][0]['formula'])),'PROOF_FORMULA_CAPTURE_MISMATCH')
    for name in ['spell.schema.json','spell-dependencies.schema.json']:
        require(proof['schema_proofs'][name]==digest(schemas['schemas/'+name]),'PROOF_SCHEMA_PIN_MISMATCH')
    receipt=json.loads(packet[folder+'/receipt.json']);header=json.loads(old_header)['spell']
    validators['schemas/player-bundle-receipt.schema.json'].validate(receipt)
    require(receipt['status']=='CANDIDATE_SCHEMA_VALID' and not receipt['blockers'] and receipt['runtime_activation'] is False and receipt['native_execution_qualified'] is False
            and receipt['source_revision']==source_pin and receipt['source_sha256']==callback['source_sha256'] and receipt['source_header']==header and receipt['registration_key']==reg and receipt['candidate_key']==candidate_key
            and receipt['remaining_mechanics'],'RECEIPT_SOURCE_OR_STATUS_MISMATCH')
    spell=json.loads(packet[folder+'/spell.json']);deps=json.loads(packet[folder+'/dependencies.json']);catalog=json.loads(packet[folder+'/catalog.json'])
    validators['schemas/spell.schema.json'].validate(spell);validators['schemas/spell-dependencies.schema.json'].validate(deps)
    require(spell['spell']['identity']=={'key':candidate_key,'revision':candidate_revision},'SPELL_IDENTITY_MISMATCH')
    require(receipt['dependencies']=={key:len(values) for key,values in deps.items()},'RECEIPT_DEPENDENCY_COUNT_MISMATCH')
    validate_dependency_closure(spell,deps,catalog,candidate_key,candidate_revision)
    source_path=source.resolve().relative_to(root.resolve()).as_posix()
    data={'player-source-candidates/'+name:body for name,body in packet.items()};data.update(schemas);data['base-r28/import-manifest.json']=base_body
    def refs(name):
        schema_name={'spell.json':'spell.schema.json','dependencies.json':'spell-dependencies.schema.json','receipt.json':'player-bundle-receipt.schema.json'}.get(Path(name).name)
        return [json.loads(schemas['schemas/'+schema_name])['$id']] if schema_name else []
    manifest={'schema':'OTERYN_INCREMENTAL_SOURCE_SPELL_IMPORT/v1','admission_status':'source_only_not_active','runtime_activation':False,'native_identity_allocation':False,'canonical_selection_changed':False,'native_execution_qualified':False,'input_provider_equivalence':False,
        'base':{'path':'imports/spells/r28/import-manifest.json','sha256':BASE_SHA,'snapshot':'base-r28/import-manifest.json'},
        'source_package':{'path':source_path+'/package-manifest.json','sha256':config['manifest_sha256']},'qualification_proof_sha256':config['proof_sha256'],'source_pins':baseline['source_pins'],'schemaRefs':baseline['schemaRefs'],'source_keys':[reg],
        'counts':{'records':1,'status_counts':{'CANDIDATE_SCHEMA_VALID':1},'abilities':1,'effects':2,'formulas':1},
        'artifacts':[{'path':name,'sha256':digest(body),'bytes':len(body),'schemaRefs':refs(name),'role':'base_manifest' if name.startswith('base-r28/') else 'archived_base_schema' if name.startswith('schemas/') else 'reviewed_precombat_source_candidate','sourcePath':'imports/spells/r28/import-manifest.json' if name.startswith('base-r28/') else 'imports/spells/r28/'+name if name.startswith('schemas/') else source_path+'/'+name.split('/',1)[1]} for name,body in sorted(data.items())],
        'limits':['Only the single reviewed '+revision+' current source pre-Combat healing candidate is copied.','Source headers/callbacks and historical BLOCKED receipt remain immutable in r28.','Typed healing/effect chronology does not qualify native legality, assets, engine numeric conversion or runtime activation.']}
    return data,manifest

def write_import(root,revision,destination,data,manifest):
    require(revision in CONFIGS,'UNSUPPORTED_PRECOMBAT_FAMILY')
    require(manifest['source_keys']==[CONFIGS[revision]['registration_key']] and manifest['source_package']['sha256']==CONFIGS[revision]['manifest_sha256'],'WRONG_PRECOMBAT_IMPORT_FAMILY')
    return write_source_only_set(root,destination,'imports/spells/'+revision,data,manifest)

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--revision',choices=sorted(CONFIGS),required=True);parser.add_argument('--source',type=Path);parser.add_argument('--prepare-only',action='store_true');args=parser.parse_args()
    data,manifest=prepare_import(ROOT,args.revision,args.source)
    print(json.dumps({'artifacts':len(data),'source_keys':manifest['source_keys']} if args.prepare_only else write_import(ROOT,args.revision,ROOT/'imports/spells'/args.revision,data,manifest),sort_keys=True))
