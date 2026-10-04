"""Concrete R61 source-native authoring extensions; no consumer/runtime admission."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import re
import project_party_summon_candidates as prior
import native_companions as native
import validate_spell as validator

HERE=Path(__file__).resolve().parent
ROOT=prior.ROOT
SCHEMA=HERE/'source-party-summon-native-extensions.schema.json'


def source(donor,path):return prior.base.source_file(Path('/workspace/spell-sources')/donor,prior.PINS[donor],path)


def proof(donor,path,symbol=None):
    raw=source(donor,path)
    result={'path':path,'revision':prior.PINS[donor],'sha256':prior.sha(raw)}
    if symbol:
        if '::' in symbol:body=prior.base.source_cpp_function(raw,symbol)
        else:
            text=raw.decode();start=text.index('function '+symbol+'(')
            following=re.search(r'(?m)^function ',text[start+1:])
            body=text[start:start+1+following.start() if following else len(text)].encode()
        result.update(symbol=symbol,scope_sha256=prior.sha(body))
    return result


def condition_parameters(declaration):
    params={}
    for method,args in declaration['calls']:
        if method=='setParameter':params[args[0].removeprefix('CONDITION_PARAM_').lower()]=args[1]
        elif method=='setFormula':params['speed_coefficients']=args
        else:raise ValueError('unrepresented condition method '+method)
    return {'condition_type':declaration['type'],'parameters':params,'replacement':'source_condition_type_id_subid_rules'}


def party(raw,text,donor):
    fixed='local baseMana' not in text
    base=int(re.search(r'local baseMana\s*=\s*(\d+)',text)[1]) if not fixed else raw['registrar']['mana']
    params={'membership':{'source':'members_then_append_leader','deduplicate':False,
                          'requires_party':True,'requires_source_list_size_greater_than_one':not fixed,'source_list_type_guard_after_leader_append':not fixed,
                          'distance_metric':'max_abs_xyz','distance_lte':36,'same_floor_required':False,
                          'min_selected_count':2},
            'conditions':[condition_parameters(c) for c in raw['conditions']],
            'combat_presentation':{'areas':raw['combats'][0].get('areas',[]),
                                   'effect':raw['combats'][0]['parameters'].get('COMBAT_PARAM_EFFECT'),
                                   'aggressive':False,'condition_application_independent_of_area':True,
                                   'execute_failure_message':'not_possible','execute_failure_effect':'poff'},
            'mana':{'registrar_base':base,'mode':'registrar_only' if fixed else 'ceil_geometric_party_size',
                    'geometric_ratio':None if fixed else .9,'power_offset':None if fixed else -1,
                    'count_multiplier':not fixed,'check_total_before_presentation':not fixed,
                    'additional_debit':'none' if fixed else 'total_minus_registrar_base',
                    'add_mana_spent':'none' if fixed else 'total_minus_registrar_base',
                    'debit_notify':False,'extra_delta_can_be_negative':not fixed,
                    'mana_spent_numeric_input_type':'none' if fixed else 'uint64_t',
                    'registrar_cost_after_success':True,'refund_on_failed_cast':False},
            'no_party_or_insufficient_members':{'message':None if fixed else 'No party members in range.','effect':'poff','return':False},
            'insufficient_mana':None if fixed else {'message':'RETURNVALUE_NOTENOUGHMANA','effect':'poff','return':False,
                                                       'order':['cancel_message','caster_poff','return_false']},
            'member_success_presentation':'magic_blue' if fixed else None,
            'per_member_commit_order':['add_condition','magic_blue'] if fixed else ['add_condition'],
            'commit_order':['select_members','reject_small_selection']+([] if fixed else ['compute_total_mana','check_total_mana'])+
                           ['execute_combat_or_fail']+([] if fixed else ['debit_extra_mana','record_extra_mana_spent'])+
                           ['apply_per_member_commits_in_member_list_order','return_true','registrar_success_cost']}
    return {'key':'party_buff','parameters':params},[proof(donor,'src/lua/functions/map/position_functions.cpp','PositionFunctions::luaPositionGetDistance')]+([] if fixed else [proof(donor,'src/lua/functions/creatures/player/player_functions.cpp','PlayerFunctions::luaPlayerAddManaSpent')])


def familiar(raw,text,donor):
    vocation=Path(raw['file']).stem.removesuffix('_familiar')
    descriptor=native._recipe('summon '+vocation+' familiar',donor)
    params=descriptor['parameters'];spellid=int(re.search(r'local spellId\s*=\s*(\d+)',text)[1])
    params['reference_spell_id']=spellid;params['shared_cooldown_identity']=spellid
    params['source_contract_version']='current-pinned-r61'
    params['selection']={'base_vocation_lookup':'FAMILIAR_ID','unknown_vocation_returns_false':True,
                         'creature_name':vocation.title()+' familiar','look_type_from_player':True,
                         'fallback_look_type_on_login':params['default_look_type']}
    params['current_helper_commit_order']=(['premium_guard','summon_count_and_account_guard','vocation_lookup',
        'compute_half_config_duration_seconds','compute_vip_cooldown','create_owned_monster',
        'apply_player_familiar_look','register_familiar_death','increase_speed_nonnegative',
        'caster_magic_blue','creature_teleport','save_unix_expiry','schedule_expiry','schedule_two_warnings']+
        (['register_party_protection_all_owned_summons'] if donor=='crystal' else [])+['apply_shared_spell_cooldown','return_true'])
    params['dynamic_inputs']={'duration':'config.FAMILIAR_TIME','vip_reduction':'config.VIP_FAMILIAR_TIME_COOLDOWN_REDUCTION',
                             'cooldown_rate':'config.RATE_SPELL_COOLDOWN','look':'player.familiar_look_type',
                             'vocation':'player.vocation_base_id','warning_handles':'player.FAMILIAR_TIMER_storage',
                             'unix_expiry':'player.kv.familiar-summon-time'}
    params['expiry']['absent_player_or_creature_returns_true']=True
    params['expiry']['warning_storage_reset']=-1
    params['warning_dispatch']['message_class']='loot'
    params['login']['register_advance_event_before_selection']=True
    params['login']['remove_look_guard']='(not_premium_and_has_look)_or_level_below_200'
    params['condition_sharing']={'spell_cooldown_subid':spellid,'owner_speed_applied_at_creation':True,
                                'future_haste_sharing':'companion_haste_source_policy','automatic_clone_all_owner_conditions':False}
    proofs=[proof(donor,'data/libs/functions/player.lua',symbol) for symbol in ('Player:getFamiliarName','Player:CreateFamiliarSpell','Player:createFamiliar')]
    proofs += [proof(donor,p) for p in ('data/libs/systems/familiar.lua','data/scripts/creaturescripts/familiar/on_login.lua',
                                     'data/scripts/creaturescripts/familiar/on_death.lua','data/scripts/creaturescripts/familiar/on_advance.lua',
                                     'config.lua.dist','src/config/configmanager.cpp')]
    return descriptor,proofs


def swift(raw,text,donor):
    return {'key':'companion_haste','parameters':{
        'source_contract_version':'current-swift-foot-r61','duration_ms':10000,
        'caster_haste_coefficients':[1.8,72,1.8,72],
        'familiar_selection':{'owned_summons_in_list_order':True,'require_monster_type_familiar_flag':True},
        'familiar_speed':{'base':'max_owner_base_and_familiar_base','multiplier':.8,'offset':-72,'allow_negative_delta':True},
        'pre_combat_apply_familiar_conditions':True,'combat_failure_returns_false_without_reverting_familiar_haste':True,
        'combat_effect':'magic_green','combat_aggressive':False,
        'wheel_grade_source':'player.upgradeSpellsWOD(Swift Foot)',
        'wheel_conditions':{'none':[{'type':'exhaust_combat','duration_ms':10000},{'type':'pacified','duration_ms':10000},
                                     {'type':'spell_group_cooldown','sub_id':1,'duration_ms':10000}],
                            'regular':[{'type':'attributes','damage_dealt_percent':50,'duration_ms':10000}],
                            'other':[]},
        'commit_order':['apply_familiar_haste','execute_combat','if_failed_return_false','read_wheel_grade','apply_grade_conditions','return_true']}},[]


def build_row(plan,fact):
    raw=fact['source_callback_facts'];donor=raw['source'];key=plan['registration_key']
    text=source(donor,raw['file']).decode();legacy=prior.build_row(plan,fact)
    if raw['name'].casefold() in prior.NAMES:
        descriptor=copy.deepcopy(legacy['proposed_native_binding']['bundle']['spell']['execution']['native_behavior'])
        descriptor['parameters']['inherit_master_attack_target']={'enabled':True,'overwrite_existing':True,'clear_when_master_has_no_target':True,'order':'after_set_master'}
        descriptor['parameters']['source_contract_version']='acquire-target-copy-r61'
        proofs=legacy['proposed_native_binding']['bounded_function_proofs']
    elif '/familiar/' in raw['file']:descriptor,proofs=familiar(raw,text,donor)
    elif '/party/' in raw['file']:descriptor,proofs=party(raw,text,donor)
    elif raw['name'].casefold()=='swift foot':descriptor,proofs=swift(raw,text,donor)
    else:
        import project_equipment_source_candidates as equipment
        descriptor=None;proofs=[proof(donor,'data/scripts/lib/register_spells.lua',symbol) for symbol in
                                  ('calculateBaseDamageHealing','calculateAttackValue','calculateMonkSpellDamage')]
    defaults,defaultproofs=prior.base.default_fields(Path('/workspace/spell-sources')/donor,prior.PINS[donor])
    source_receipt=json.loads(((ROOT/legacy['source_header_path']).parent/'receipt.json').read_bytes())
    identity={'key':source_receipt['candidate_key'],'revision':'source-player-r61'}
    spell=prior.base.fill_header(legacy['source_header'],defaults,identity,raw['registrar'])
    deps={'abilities':[],'effects':[],'formulas':[]}
    if descriptor is None:
        deps,routes=equipment.make_dependencies(raw,spell,donor,text)
        equipment_key,params=equipment.source_parameters(raw,donor,text,routes)
        descriptor={'key':equipment_key,'parameters':params}
    descriptor['parameters']['source_model']='r61-'+descriptor['key']
    spell['execution']={'native_behavior':descriptor}
    if spell['carrier']=='rune':spell['targeting'].setdefault('range_tiles',0)
    bundle={'spell':spell}
    return {'registration_key':key,'source_revision':prior.PINS[donor],'source_sha256':fact['source_sha256'],
            'source_header_path':legacy['source_header_path'],'source_header_sha256':legacy['source_header_sha256'],
            'status':'CANDIDATE_SCHEMA_VALID','target_schema_family':'private_source_complete_v2','bundle':bundle,'dependencies':deps,
            'catalog':{'definitions':prior.base.item_references(bundle)},'source_proofs':[proof(donor,raw['file'])]+proofs,
            'engine_default_proofs':defaultproofs,'native_data_model_complete':True,'required_operations_unrepresented':[],
            'runtime_activation':False,'native_execution_qualified':False,'native_identity_allocation':False,
            'canonical_selection_changed':False,'input_provider_equivalence':False,'reader_acceptance_qualified':False,
            'consumer_contract_implemented':False,'source_consumer_implemented':False,'authoring_contract_extension_pending':True,
            'source_alias_to_existing_native_profile':False}


def build(include_equipment=True):
    plans,facts=prior.records();plans=[p for p in plans if facts[p['registration_key']]['source_callback_facts']['name'].casefold()!='summon creature']
    if not include_equipment:plans=[p for p in plans if facts[p['registration_key']]['source_callback_facts']['name'].casefold()!='devastating knockout']
    rows=[build_row(p,facts[p['registration_key']]) for p in plans]
    return {'schema':'OTERYN_PARTY_SUMMON_SOURCE_EXTENSIONS/v1','records':len(rows),'rows':rows,
            'runtime_activation':False,'native_execution_qualified':False,'consumer_contract_implemented':False}


def closed_schema(value):
    if isinstance(value,dict):
        return {'type':'object','additionalProperties':False,'required':sorted(value),
                'properties':{key:closed_schema(item) for key,item in value.items()}}
    if isinstance(value,list):
        if not value:return {'type':'array','items':False,'minItems':0,'maxItems':0}
        return {'type':'array','prefixItems':[closed_schema(item) for item in value],
                'items':False,'minItems':len(value),'maxItems':len(value)}
    kind='null' if value is None else 'boolean' if isinstance(value,bool) else 'integer' if isinstance(value,int) else 'number' if isinstance(value,float) else 'string'
    return {'type':kind,'const':value}


def private_schema(packet):
    result=copy.deepcopy(validator.SCHEMAS['spell.schema.json'])
    result['$id']='urn:oteryn:source-party-summon-native-extensions:1'
    descriptors=[]
    for row in packet['rows']:
        desc=row['bundle']['spell']['execution']['native_behavior']
        if desc not in descriptors:descriptors.append(desc)
    result['$defs']['nativeBehavior']={'oneOf':[closed_schema(desc) for desc in descriptors]}
    # Current source party registrar charges the base mana after success. Legacy party
    # authoring had mana=0 for its all-native cost model; this private contract preserves it.
    result['$defs']['spell']['allOf']=[rule for rule in result['$defs']['spell']['allOf'] if
        rule.get('if',{}).get('properties',{}).get('execution',{}).get('properties',{}).get('native_behavior',{}).get('properties',{}).get('key',{}).get('const')!='party_buff']
    return result


def validate(packet):
    if packet!=build():raise ValueError('exact source extension rebuild mismatch')
    schema=private_schema(packet)
    check=validator.Draft202012Validator(schema,registry=validator.REGISTRY)
    for row in packet['rows']:
        check.validate(row['bundle'])
        errors=validator.structural('spell-dependencies.schema.json',row['dependencies'])
        if errors:raise ValueError(str(errors))
        if row['runtime_activation'] or row['reader_acceptance_qualified']:raise ValueError('consumer pending')


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--out',type=Path,required=True);args=parser.parse_args()
    packet=build();validate(packet);out=args.out;out.mkdir(parents=True,exist_ok=True)
    prior.write_json(SCHEMA,private_schema(packet));prior.write_json(out/'source-native-extensions.json',packet)
    prior.write_json(out/'lane-audit.json',{'records':[{k:v for k,v in r.items() if k not in ('bundle','dependencies','catalog')} for r in packet['rows']]})
    for row in packet['rows']:
        folder=out/row['registration_key'].split('/')[0]/prior.sha(row['registration_key'].encode())[:16];folder.mkdir(parents=True,exist_ok=True)
        (folder/'source-header.json').write_bytes((ROOT/row['source_header_path']).read_bytes())
        for name,value in [('spell',row['bundle']),('dependencies',row['dependencies']),('catalog',row['catalog'])]:prior.write_json(folder/(name+'.json'),value)
        archived=ROOT/row['source_header_path']
        receipt=json.loads((archived.parent/'receipt.json').read_bytes())
        receipt.update(status='CANDIDATE_SCHEMA_VALID',blockers=[],native_execution_qualified=False,
            dependencies={family:len(values) for family,values in row['dependencies'].items()},item_owner_bindings_required=row['catalog']['definitions'],
            schema_and_semantic_validation_errors=[],conversion_notes=['Concrete R61 source parameter extension; private pending consumer contract.'],
            remaining_mechanics=[{'source_field':'source.native_consumer','reason':'Authoring contract extension pending; consumer/runtime admission not qualified.'}])
        validator.Draft202012Validator(prior.base.receipt_schema(),registry=validator.REGISTRY).validate(receipt)
        prior.write_json(folder/'receipt.json',receipt)
        prior.write_json(folder/'projection-receipt.json',{k:v for k,v in row.items() if k not in ('bundle','dependencies','catalog')})
    prior.write_json(out/'import-summary.json',{'records':packet['records'],'target_schema_family':'private_source_complete_v2','authoring_contract_extension_pending':True,
                     'source_consumer_implemented':False,'auditrecords':packet['records'],'full_source_registration_candidates':packet['records'],
                     'status_counts':{'CANDIDATE_SCHEMA_VALID':packet['records']},'source_extension_data_candidates':packet['records'],
                     'runtime_activation':False,'native_execution_qualified':False,'consumer_contract_implemented':False,
                     'records_index':[{'registration_key':r['registration_key'],'source_revision':r['source_revision'],'source_sha256':r['source_sha256'],
                       'status':r['status'],'runtime_activation':False,'native_execution_qualified':False,
                       'authoring_contract_extension_pending':True,'source_consumer_implemented':False,
                       'snapshot':r['registration_key'].split('/')[0]+'/'+prior.sha(r['registration_key'].encode())[:16]} for r in packet['rows']]})
    prior.write_json(out/'projection-proof.json',{'records':packet['records'],'target_schema_family':'private_source_complete_v2','runtime_activation':False,'native_execution_qualified':False,
                     'native_identity_allocation':False,'canonical_selection_changed':False,'input_provider_equivalence':False,
                     'authoring_contract_extension_pending':True,'source_consumer_implemented':False,
                     'schema_proofs':{str(SCHEMA.relative_to(ROOT)):prior.sha(SCHEMA.read_bytes())},
                     'policy_proofs':[{'path':prior.POLICY,'sha256':prior.sha((ROOT/prior.POLICY).read_bytes()),
                       'exact_rows':[line for line in (ROOT/prior.POLICY).read_text().splitlines() if "creature's `summoning.mana_cost`" in line or "target's `summoning.mana_cost`" in line]}],
                     'input_proofs':{str(prior.WORKLIST.relative_to(ROOT)):prior.sha(prior.WORKLIST.read_bytes()),
                       str((prior.R28/'source-callback-facts.jsonl.gz').relative_to(ROOT)):prior.sha((prior.R28/'source-callback-facts.jsonl.gz').read_bytes())}})
    (out/'README.md').write_text('''# R61 concrete party/summon source DATA models

26 source registrations: four acquired-summon runes, ten familiar wrappers, ten party support casts, Canary Swift Foot, Crystal Devastating Knockout. Current donor pins are Canary 04b83b51 and Crystal 00ce02a5; only existing local objects/captures are read. Two Summon Creature registrations are already complete in R57 and excluded.

Each registration has a byte-exact R28 source-header, an actual Spell bundle, standard dependencies/catalog, standard receipt and separate source projection receipt. All 26 target models validate under the composed private source-complete authoring schema. Eight focused tests verify real schema shapes, precise population, source operations and closed extension fields.

The private native schema exposes concrete typed closed parameter contracts for acquire_summon target inheritance (including clearing), familiar_summon current helper lifecycle and dynamic config providers, party_buff source membership/condition SUBID and two-stage mana commitment, companion_haste current Swift Foot wheel branches, and equipment_attack Devastating Knockout elemental-bond dispatch with three standard Ability/Effect/Formula routes. Source positions use max absolute X/Y/Z distance, without same-floor or deduplication shortcuts. Crystal Enlighten Party uses fixed registrar mana and per-member presentation. Failed Swift Foot Combat does not revert already applied familiar haste. Devastating Knockout party-heal comment blocks have no executable effect.

Canonical cost selection follows the already accepted C.2 summoning.mana_cost decision; original raw source headers and source hashes remain separate. Every new native parameter contract requires consumer implementation and qualification. CANDIDATE_SCHEMA_VALID means complete local authoring DATA. Runtime activation, native execution, input-provider equivalence, native identity allocation, canonical selection and reader acceptance remain false. No server files or existing v1 schemas are changed.
''')
    prior.write_json(out/'package-manifest.json',{'schema_path':str(SCHEMA.relative_to(ROOT)),'schema_sha256':prior.sha(SCHEMA.read_bytes()),
                     'files':{str(p.relative_to(out)):prior.sha(p.read_bytes()) for p in sorted(out.rglob('*')) if p.is_file() and p.name!='package-manifest.json'}})

if __name__=='__main__':main()
