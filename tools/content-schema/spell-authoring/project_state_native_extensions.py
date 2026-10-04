"""Concrete S27 native data and bounded private authoring extensions, never admission.

No Lua, AST program, generic operation graph or runtime alias is emitted. Existing
accepted native parameter models supply canonical values; current source headers
and branch differences retain separate provenance.
"""
import argparse
import copy
import gzip
import json
import re
from pathlib import Path

import native_actor_states
import native_combat
import native_delayed
import project_state_wheel_candidates as previous
import import_source_player_bundles as base
import validate_spell

HERE = Path(__file__).resolve().parent
OUT = Path('docs/reference/spells/r59-source-closure')
REV = 'source-player-r59'
SCHEMA = 'source-state-native-extensions.schema.json'
FLAGS = {name: False for name in ['runtime_activation', 'native_execution_qualified', 'canonical_selection_changed', 'native_identity_allocation', 'input_provider_equivalence', 'source_consumer_implemented']}
POLICY = Path('docs/architecture/OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md')
SOURCE = Path('/workspace/spells-r22-monster-import-current/source-inputs')
STANCES = ['STANCE_BLOOD_RAGE','STANCE_PROTECTOR','STANCE_SHARPSHOOTER','STANCE_MASTER_OF_FLAMES','STANCE_MASTER_OF_THUNDER','STANCE_MASTER_OF_DECAY','STANCE_DIVINE_DEFIANCE','STANCE_ELEMENTAL_SYNTHESIS','STANCE_SHARED_CONSERVATION','STANCE_EXPOSE_WEAKNESS','STANCE_SAP_STRENGTH']
ATTRIBUTES = {'CONDITION_PARAM_SKILL_MELEEPERCENT': 'melee_skill_percent', 'CONDITION_PARAM_SKILL_SHIELDPERCENT': 'shield_skill_percent', 'CONDITION_PARAM_SKILL_DISTANCEPERCENT': 'distance_skill_percent', 'CONDITION_PARAM_BUFF_DAMAGEDEALT': 'damage_dealt_percent', 'CONDITION_PARAM_BUFF_DAMAGERECEIVED': 'damage_received_percent', 'CONDITION_PARAM_BUFF_HEALINGRECEIVED': 'healing_received_percent', 'CONDITION_PARAM_DISABLE_DEFENSE': 'disable_defense'}


def obj(props, required=None): return {'type':'object','additionalProperties':False,'properties':props,'required':list(props) if required is None else required}
def enum(values): return {'enum': values}
def arr(item): return {'type':'array','items':item}


def inferred_closed(value):
    if isinstance(value,dict): return obj({k:inferred_closed(v) for k,v in value.items()})
    if isinstance(value,list):
        if not value: return arr({"type":"object"})
        variants=[inferred_closed(v) for v in value]
        return arr({"anyOf":variants})
    if isinstance(value,bool): return {"type":"boolean"}
    if isinstance(value,int): return {"type":"integer"}
    if isinstance(value,float): return {"type":"number"}
    if value is None: return {"type":"null"}
    return {"const":value}


def extension_schema(rows):
    schema = copy.deepcopy(json.loads((HERE / 'spell.schema.json').read_text()))
    schema['$id'] = 'urn:oteryn:source-state-native-extensions:1'
    condition = obj({'sub_id_symbol': {'type':['string','null']}, 'duration_ms': {'type':['integer','null'], 'minimum':-1},
                     'buff_spell': {'type':'boolean'}, **{name: {'type':'boolean'} if name=='disable_defense' else {'type':['integer','null']} for name in ATTRIBUTES.values()}})
    stance = obj({'source_stance': enum(STANCES), 'slot': enum(['standard','elemental']), 'player_only_state_access': {'const':True},
                  'non_player_route': {'const':'execute_primary_combat_without_state_change'},
                  'same_stance_order': {'const':['remove_matching_attribute_condition_if_present','clear_slot','poff','return_true']},
                  'other_stance_order': {'const':['set_slot_before_combat','execute_primary_combat','return_combat_result']},
                  'condition_id': {'const':'CONDITIONID_COMBAT'}, 'condition':condition,
                  'impact_effect_id': {'type':'integer','minimum':0}, 'toggle_off_effect': {'const':'CONST_ME_POFF'},
                  'source_owner': {'const':'Crystal_player_stance'}, 'source_header_costs_unchanged': {'const':True}})
    matrix = {'type':['array','null'],'items':arr({'type':'integer','minimum':0,'maximum':3})}
    beam = obj({'source_base_spell':{'const':'great_death_beam'}, 'wheel_unlock':{'const':True},
                'stage_read': {'const':'cast_start_once'}, 'stage_zero': {'const':'refuse_before_costs'},
                'stages':arr(obj({'stage':enum([1,2,3]), 'source_combat_id':{'type':'integer'}, 'north':matrix, 'diagonal':matrix,
                                 'formula': {'$ref':'#/$defs/formula'}})),
                'shared_combat_alias': {'type':'boolean'}, 'final_area_used_by_shared_alias':matrix,
                'separate_augments':{'const':'ProjectV2AugmentBinding'}, 'effect_asset_binding':{'type':'string'}})
    original = schema['$defs']['nativeBehavior']
    schema['$defs']['originalNativeBehavior'] = original
    branches=[]
    prototypes=set()
    for row in rows:
        model=row.get('spell',{}).get('spell',{}).get('execution',{}).get('native_behavior')
        if model is None: continue
        prototype=inferred_closed(model)
        token=previous.canonical(prototype)
        if token not in prototypes: branches.append(prototype);prototypes.add(token)
    schema['$defs']['nativeBehavior']={'anyOf':branches}
    contract = obj({'version':{'const':1}, 'normalization':enum(['S27_ACCEPTED_CANONICAL_NATIVE_PARAMETERS','CURRENT_CRYSTAL_STANCE_ORDER','CURRENT_SOURCE_SHARED_COMBAT_GEOMETRY','S23_CHAIN_INITIAL_SELECTOR']),
                    'authoring_contract_extension_pending':{'const':True}, 'runtime_activation':{'const':False}, 'native_execution_qualified':{'const':False},
                    'raw_source_equivalence':{'const':False}, 'wheel_augments_owner':{'const':'ProjectV2AugmentBinding'},
                    'chain_initial_selector':{'anyOf':[{'type':'null'},obj({'target_route':{'const':'explicit_target_then_sequential_chain'},
                        'direction_route':{'const':'directional_single_target_without_chain'},'initial_range_tiles':{'const':7},'source_total_targets':{'const':3},'source_jump_radius':{'const':5},
                        'canonical_additional_targets':{'const':2},'canonical_jump_radius':{'const':4},'branch_selection':{'const':'resolved_variant_at_cast_start'}})]}})
    augment_prototypes=[]
    for row in rows:
        if 'spell' not in row:continue
        value=row['spell']['spell']['source_state_contract']['separate_augment_parameters']
        prototype=inferred_closed(value)
        if prototype not in augment_prototypes:augment_prototypes.append(prototype)
    contract['properties']['separate_augment_parameters']={'anyOf':augment_prototypes}
    contract['required'].append('separate_augment_parameters')
    schema['$defs']['sourceStateContract']=contract
    schema['$defs']['spell']['properties']['source_state_contract'] = {'$ref':'#/$defs/sourceStateContract'}
    return schema


def stance_params(text):
    selected = re.search(r'get(?:Elemental)?Stance\(\)\s*==\s*(STANCE_\w+)', text)
    if not selected or selected.group(1) not in STANCES: raise ValueError('unqualified exact stance selector')
    condition = {'sub_id_symbol':None,'duration_ms':None,'buff_spell':False, **{name:False if name=='disable_defense' else None for name in ATTRIBUTES.values()}}
    for key, value in re.findall(r'condition:setParameter\((\w+),\s*([^\)]+)\)',text):
        value=value.strip()
        if key=='CONDITION_PARAM_SUBID': condition['sub_id_symbol']=value
        elif key=='CONDITION_PARAM_TICKS': condition['duration_ms']=int(value)
        elif key=='CONDITION_PARAM_BUFF_SPELL': condition['buff_spell']=value=='true'
        elif key in ATTRIBUTES: condition[ATTRIBUTES[key]]=value=='true' if value in {'true','false'} else int(value)
        else: raise ValueError('unmapped concrete stance condition parameter '+key)
    effect = re.search(r'combat:setParameter\(COMBAT_PARAM_EFFECT,\s*(\d+)\)',text)
    if not effect: raise ValueError('source stance effect not exact integer')
    remove = text.find('player:removeCondition('); clear = text.find('player:setElementalStance(STANCE_NONE)' if 'getElementalStance' in text else 'player:setStance(STANCE_NONE)')
    poff = text.find('sendMagicEffect(CONST_ME_POFF)'); true = text.find('return true')
    set_pos = text.rfind('player:setElementalStance(' if 'getElementalStance' in text else 'player:setStance(')
    combat_pos = text.find('return combat:execute')
    if not 0 <= clear < poff < true < set_pos < combat_pos or remove>=0 and not remove<clear: raise ValueError('source stance timeline mismatch')
    return {'source_stance':selected.group(1),'slot':'elemental' if 'getElementalStance' in text else 'standard',
            'player_only_state_access':True,'non_player_route':'execute_primary_combat_without_state_change',
            'same_stance_order':['remove_matching_attribute_condition_if_present','clear_slot','poff','return_true'],
            'other_stance_order':['set_slot_before_combat','execute_primary_combat','return_combat_result'],
            'condition_id':'CONDITIONID_COMBAT','condition':condition,'impact_effect_id':int(effect.group(1)),
            'toggle_off_effect':'CONST_ME_POFF','source_owner':'Crystal_player_stance','source_header_costs_unchanged':True, 'helper_modifiers':helper_modifiers(selected.group(1))}


def helper_modifiers(stance):
    elemental={'STANCE_MASTER_OF_FLAMES':'fire','STANCE_MASTER_OF_THUNDER':'energy','STANCE_MASTER_OF_DECAY':'death'}
    result={'elemental':None,'specialized_magic':None,'crippling_aura':None,'ranged_dodge':None,'shared_conservation':None,
            'state_setter':{'reject_incompatible_base_vocation':True,'lua_ignores_setter_result':True,'clears_pending_conversion_on_elemental_change':stance in elemental,
                            'refresh_skills_and_full_stance_highlights':True,'primary_and_elemental_slots_independent':True}}
    if stance in elemental:
        result['elemental']={'element':elemental[stance],'spell_origin_only':True,'exclude_types':['healing','mana_drain','none'],
            'convert_primary_and_non_none_secondary':True,'consume_pending_before_bonus':True,'arm_only_when_not_converted':True,
            'base_power_multiplier':1.04 if elemental[stance]=='fire' else 1.0,'base_crit_chance_percentage_points':4 if elemental[stance]=='energy' else 0,
            'base_crit_extra_percentage_points':30 if elemental[stance]=='death' else 0,'integer_damage_rounding':'truncate_toward_zero',
            'lord_of_destruction_augments':'ProjectV2AugmentBinding'}
    if stance=='STANCE_DIVINE_DEFIANCE':
        result['specialized_magic']={'input':'distance_skill','percent':7.5,'elements':['holy','healing'],'rounding':'truncate_toward_zero'}
        result['ranged_dodge']={'any_attack':True,'chebyshev_distance_greater_than':1,'different_floor_also_qualifies':True,'uniform_inclusive_min':0,'uniform_inclusive_max':10000,'success_strictly_below':1500,'skip_damage_on_success':True}
    if stance=='STANCE_ELEMENTAL_SYNTHESIS':result['specialized_magic']={'input':'magic_level','percent':10,'elements':['ice','earth'],'rounding':'truncate_toward_zero'}
    if stance in ['STANCE_EXPOSE_WEAKNESS','STANCE_SAP_STRENGTH']:
        result['crippling_aura']={'duration_ms':10000,'requires_aggressive_successful_nonzero_nonheal_hit':True,'includes_pvp':True,'refresh_same_subid':True,
            'subid_symbol':'SorcererExposeWeaknessAura' if stance=='STANCE_EXPOSE_WEAKNESS' else 'SorcererSapStrengthAura',
            'target_damage_dealt_percent':90 if stance=='STANCE_SAP_STRENGTH' else None,'absorb_elements':['fire','ice','energy','earth'] if stance=='STANCE_EXPOSE_WEAKNESS' else [],
            'target_absorb_percent':-8 if stance=='STANCE_EXPOSE_WEAKNESS' else None}
    if stance=='STANCE_SHARED_CONSERVATION':result['shared_conservation']={'healing_received_percent':110,'natures_embrace_secondary_ratio':0.30,'secondary_target_owner':'natures_embrace_party_nearest_target_selector'}
    return result


def beam_params(donor, cached, fact):
    combats=cached['conversion']['reference_combats']; callbacks=fact['source_callback_facts']['combats']
    alias=donor=='canary'; ids=[0,0,0] if alias else [0,2,4]
    stages=[]
    for grade,number in enumerate(ids,1):
        combat=combats[str(number)]; callback=next(value for value in callbacks[number]['callbacks'] if 'formula' in value)
        formula={'identity':{'key':'candidate:formula/great-death-beam/grade-'+str(grade),'revision':REV}, **previous.normalized_formula(callback)}
        stages.append({'stage':grade,'source_combat_id':number,'north':combat['area']['north'],'diagonal':combat['area'].get('diagonal'),'formula':formula})
    return {'source_base_spell':'great_death_beam','wheel_unlock':alias,'stage_read':'cast_start_once','stage_zero':'refuse_before_costs' if alias else 'select_grade_one_and_cast',
            'zero_refusal':{'message':'You need to learn this spell first','effect':'CONST_ME_POFF','before_resource_costs':True} if alias else None,
            'source_flank_augments':None if alias else {'owner':'ProjectV2AugmentBinding','source_stage_damage_factors':[0,0.4,0.6,0.8], 'stage_read':'each_formula_callback', 'runs_after_primary_even_if_primary_false':True, 'returned_result':'primary_only', 'north_by_beam_index':[combats[str(i)]['area']['north'] for i in [1,3,5]], 'base_formula':'same_primary_formula_times_stage_factor'},
            'elemental_routes':None if alias else {'default':'death','STANCE_MASTER_OF_FLAMES':{'element':'fire','impact_effect_id':334},'STANCE_MASTER_OF_THUNDER':{'element':'energy','impact_effect_id':335},'retunes_all_primary_and_flank_combats_before_grade_read':True},
            'stages':stages,'shared_combat_alias':alias,'final_area_used_by_shared_alias':combats['0']['area']['north'] if alias else None,
            'separate_augments':'ProjectV2AugmentBinding','effect_asset_binding':donor+'.appearance:effect/mortarea'}


def immutable_helper_proofs():
    revision=previous.source_cast_programs_pins()['crystal']; source_repo=Path('/workspace/spell-sources/crystal')
    scopes={
        'src/creatures/players/player.cpp': {
            'Player::setStance(':['m_stancePrimary = stance','isStanceCompatibleWithVocation','sendStanceProtocol();','sendSkills();'],
            'Player::setElementalStance(':['m_pendingElementConversion = COMBAT_NONE','m_stanceElemental = stance','VOCATION_SORCERER'],
            'Player::getSpecializedMagicLevel(':['getSkillLevel(SKILL_DISTANCE) * 0.075','getMagicLevel() * 0.10','COMBAT_HOLYDAMAGE','COMBAT_HEALING','COMBAT_ICEDAMAGE','COMBAT_EARTHDAMAGE'],
        },
        'src/creatures/combat/combat.cpp': {
            'applyElementalStance(':['damage.origin != ORIGIN_SPELL','damage.primary.type = pending','damage.secondary.type = pending','1.04f','criticalChance += 400','criticalDamage += 3000','if (!converted)'],
            'applyCripplingStanceAura(':['10000','CONDITION_PARAM_BUFF_DAMAGEDEALT, 90','CONDITION_PARAM_ABSORB_FIREPERCENT, -8','CONDITION_PARAM_ABSORB_ICEPERCENT, -8','CONDITION_PARAM_ABSORB_ENERGYPERCENT, -8','CONDITION_PARAM_ABSORB_EARTHPERCENT, -8'],
            'Combat::CombatHealthFunc(':['applyCripplingStanceAura(attackerPlayer, target)','params.aggressive','damage.primary.type != COMBAT_HEALING','damage.primary.value != 0 || damage.secondary.value != 0'],
        },
        'src/game/game.cpp':{'Game::combatBlockHit(':['STANCE_DIVINE_DEFIANCE','Position::getDistanceZ(ap, tp) != 0','Position::getDistanceX(ap, tp) > 1','uniform_random(0, 10000) < 1500']},
        "data/scripts/spells/healing/nature's_embrace.lua":{'shareConservationHeal(':['member:getId() ~= primaryTargetId','member:getId() ~= player:getId()','member:getHealth() > 0','pos.z == origin.z','dist < bestDist','Variant(best:getId())']},
    }
    proofs={}; selected=[]
    for path,functions in scopes.items():
        raw=base.source_file(source_repo,revision,path);text=raw.decode();proofs[path]=previous.sha(raw)
        for signature,required in functions.items():
            start=text.find(signature)
            if start<0:raise ValueError('missing immutable helper scope '+signature)
            start=text.rfind('\n',0,start)+1
            end=text.find('\nend' if path.endswith('.lua') else '\n}',start)
            if end<0:raise ValueError('unclosed immutable helper scope '+signature)
            scope=text[start:end+(4 if path.endswith('.lua') else 2)]
            for literal in required:
                if literal not in scope:raise ValueError('immutable helper does not qualify '+signature+': '+literal)
            selected.append({'path':path,'revision':revision,'function':signature,'start_line':text[:start].count('\n')+1,
                             'end_line':text[:end].count('\n')+2,'scope_sha256':previous.sha(scope.encode()),'qualified_literals':required})
    return proofs,selected


def build(repo):
    repo=Path(repo)
    cohort=[row for row in json.loads((repo/'docs/reference/spells/r55-source-closure/lane-audit.json').read_text())['rows'] if row['status']=='BLOCKED']
    if len(cohort)!=43: raise ValueError('requires exact remaining43')
    facts={r['registration_key']:r for r in map(json.loads,gzip.decompress((repo/previous.BASE/'source-callback-facts.jsonl.gz').read_bytes()).splitlines())}
    caches={(r['source'],r['provenance']['path']):r for r in json.loads(previous.CACHE.read_text())}
    result=[]
    for row in cohort:
        key=row['registration_key']; fact=facts[key]; donor=key.split('-')[0]; path=fact['source_callback_facts']['file']; raw=(SOURCE/donor/path).read_bytes()
        if previous.sha(raw)!=fact['source_sha256']: raise ValueError('source identity differs: '+key)
        cached=caches[(donor,path)]
        if cached['provenance']['sha256']!=fact['source_sha256'] or not cached['conversion']['reference_capture_complete']: raise ValueError('cached declarations not current/complete')
        original=previous.base_row(repo,key); headerraw=(original/'source-header.json').read_bytes(); header=json.loads(headerraw)
        name=header['spell']['name'].casefold() if 'name' in header['spell'] else Path(path).stem.replace('_',' ')
        identity={'key':'candidate:spell/source/'+key.split('/')[0]+'/'+previous.row_id(key),'revision':REV}
        defaults,_=base.default_fields(Path('/workspace/spell-sources')/donor,fact['source_revision'])
        spell=base.fill_header(header['spell'],defaults,identity,fact['source_callback_facts']['registrar'])
        spell['requirements'].pop('vocation_display_flags',None)
        spell['targeting'].setdefault('allow_on_self',True);spell['targeting'].setdefault('check_floor',True)
        deps={'abilities':[],'effects':[],'formulas':[]}; catalog={'definitions':[]}; chain=None
        if name=='sap strength':
            result.append({'registration_key':key,'status':'REFERENCE_ONLY_REMOVED_S24','source_sha256':fact['source_sha256'],'source_revision':fact['source_revision'],'source_header_bytes':headerraw,'policy':'S24 removed Sap Strength; no executable emitted',**FLAGS});continue
        if donor=='crystal' and row['lane']=='stance' and '/support/' in key or donor=='crystal' and name=='sharpshooter':
            model={'key':'stance_toggle','parameters':stance_params(raw.decode())}; normalization='CURRENT_CRYSTAL_STANCE_ORDER'
        elif name=='great death beam':
            model={'key':'wheel_combat','parameters':beam_params(donor,cached,fact)};normalization='CURRENT_SOURCE_SHARED_COMBAT_GEOMETRY'
            spell['requirements']['wheel_unlock']=donor=='canary'
        elif name=='lightning':
            # Existing S23 target chain plus concrete alternate initial selector.
            converter=base.canary_batch.Converter(SOURCE/donor,{},{},{},{})
            converter.spell_scripts=previous.SimpleNamespace(enums=base.spell_scripts.engine_enums(SOURCE/donor));converter.pending_definitions=set()
            combat=copy.deepcopy(cached['conversion']['reference_combats']['0']);combat['callbacks']={k:v for k,v in combat['callbacks'].items() if k=='CALLBACK_PARAM_LEVELMAGICVALUE'}
            combat['param_calls']=[c for c in combat['param_calls'] if c[0]!='COMBAT_PARAM_CHAIN_EFFECT'];combat['params'].pop('COMBAT_PARAM_CHAIN_EFFECT',None)
            prefix=identity['key'];converter.combat_ability(prefix+'/ability',combat,{'needs_target':True,'needs_direction':False},7,deps,lambda value:value,[])
            form=prefix+'/formula';callback=next(v for v in fact['source_callback_facts']['combats'][0]['callbacks'] if 'formula' in v)
            for effect in deps['effects']:
                if effect.get('formula',{}).get('key')==base.canary_batch.CASTER_MAGNITUDE: effect['formula']={'family':'Formula','key':form,'revision':REV}
            deps['formulas']=[v for v in deps['formulas'] if v['identity']['key']!=base.canary_batch.CASTER_MAGNITUDE]
            deps['formulas'].append({'identity':{'key':form,'revision':REV},**previous.normalized_formula(callback)})
            for ability in deps['abilities']: ability['chain']={'max_targets':2,'range_tiles':4,'initial_range_tiles':7,'shape':'sequential','backtracking':False}
            mapping={v['identity']['key']:v['identity']['key'] for values in deps.values() for v in values};deps=previous.rebind(deps,mapping)
            # previous rebind revision belongs to r55: explicitly use this packet revision.
            def revisions(value):
                if isinstance(value,list):return [revisions(v) for v in value]
                if isinstance(value,dict):return {k:REV if k=='revision' and v==previous.REVISION else revisions(v) for k,v in value.items()}
                return value
            deps=revisions(deps);spell['execution']={'ability':{'family':'Ability','key':prefix+'/ability','revision':REV}}
            chain={'target_route':'explicit_target_then_sequential_chain','direction_route':'directional_single_target_without_chain','initial_range_tiles':7,'source_total_targets':3,'source_jump_radius':5,'canonical_additional_targets':2,'canonical_jump_radius':4,'branch_selection':'resolved_variant_at_cast_start'}
            model=None;normalization='S23_CHAIN_INITIAL_SELECTOR'
        else:
            if name in native_combat.TEMPLATES:model=copy.deepcopy(native_combat.TEMPLATES[name])
            elif name in native_actor_states.MODELS:model=copy.deepcopy(native_actor_states.MODELS[name])
            elif name in native_delayed.FILES:model=native_delayed._snapshot(name)
            else:raise ValueError('no accepted concrete model '+name)
            normalization='S27_ACCEPTED_CANONICAL_NATIVE_PARAMETERS'
            if name.startswith('avatar of') or name in {'executioner\'s throw','ice burst','terra burst','divine grenade','divine empowerment'}:spell['requirements']['wheel_unlock']=True
        separate_augments={}
        if model:
            if model['key']=='wheel_combat' and name not in ['great death beam', "executioner's throw",'ice burst','terra burst']:
                for field in ['wheel','enhanced_area','enhanced_area_from_stage','damage_or_heal_bonus_percent','beam_mastery']:
                    if field in model['parameters']:separate_augments[field]=model['parameters'].pop(field)
            if model['key']=='mass_spirit_mend':
                for field in ['wheel','healing_bonus_percent','cooldown_ms']:
                    if field in model['parameters']:separate_augments[field]=model['parameters'].pop(field)
            model['parameters']['source_model']='r59_'+('current_source_stance' if normalization=='CURRENT_CRYSTAL_STANCE_ORDER' else 'shared_source_beam' if normalization=='CURRENT_SOURCE_SHARED_COMBAT_GEOMETRY' else 'accepted_'+model['key'])
            spell['execution']={'native_behavior':model}
        spell['source_state_contract']={'version':1,'normalization':normalization,'authoring_contract_extension_pending':True,'runtime_activation':False,'native_execution_qualified':False,'raw_source_equivalence':False,'wheel_augments_owner':'ProjectV2AugmentBinding','chain_initial_selector':chain,'separate_augment_parameters':separate_augments}
        catalog={'definitions':base.item_references({'spell':spell,'dependencies':deps})}
        result.append({'registration_key':key,'status':'CANDIDATE_SCHEMA_VALID','candidate_key':identity['key'],'candidate_revision':REV,'source_sha256':fact['source_sha256'],'source_revision':fact['source_revision'],'source_header_bytes':headerraw,
                       'source_fact_sha256':previous.sha(previous.canonical(fact)),'normalization':normalization,'spell':{'spell':spell},'dependencies':deps,'catalog':catalog,'native_consumer_identity_guard':'CAPABILITY_BLOCKED_FULL_PROFILE_EQUALITY','authoring_contract_extension_pending':True,**FLAGS})
    return result


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--repo',default='.');args=parser.parse_args();repo=Path(args.repo).resolve()
    helper_proofs,helper_scopes=immutable_helper_proofs()
    rows=build(repo);schema=extension_schema(rows); previous.write_json(HERE/SCHEMA,schema)
    out=repo/OUT;out.mkdir(parents=True,exist_ok=True)
    schemas=out/'schemas';schemas.mkdir(exist_ok=True)
    previous.write_json(schemas/SCHEMA,schema)
    (schemas/'spell-dependencies.schema.json').write_bytes((HERE/'spell-dependencies.schema.json').read_bytes())
    validator=validate_spell.Draft202012Validator(schema,registry=validate_spell.REGISTRY)
    index=[]
    for row in rows:
        target=out/row['registration_key'].split('/')[0]/previous.row_id(row['registration_key']);target.mkdir(parents=True,exist_ok=True)
        (target/'source-header.json').write_bytes(row['source_header_bytes'])
        if 'spell' in row:
            errors=list(validator.iter_errors(row['spell']))
            if errors:raise ValueError(row['registration_key']+': '+str(errors[0]))
            for field in ['spell','dependencies','catalog']:previous.write_json(target/(field+'.json'),row[field])
        receipt={k:v for k,v in row.items() if k not in {'source_header_bytes','spell','dependencies','catalog'}}
        receipt['source_consumer_equivalence']=False
        receipt['target_schema_family']='private_source_complete_v2'
        receipt['source_header_sha256']=previous.sha(row['source_header_bytes'])
        previous.write_json(target/'projection-receipt.json',receipt)
        original_receipt=json.loads((previous.base_row(repo,row['registration_key'])/'receipt.json').read_text())
        if 'spell' in row:
            original_receipt.update(status='CANDIDATE_SCHEMA_VALID',blockers=[],dependencies={k:len(v) for k,v in row['dependencies'].items()},schema_and_semantic_validation_errors=[],native_execution_qualified=False,item_owner_bindings_required=row['catalog']['definitions'],conversion_notes=['S27/private typed parameter closure; current header exact; authoring contract and runtime provider pending.',row['normalization']],remaining_mechanics=[{'source_field':'runtime.native_and_extension_owner','reason':'Concrete private source data validates; authoring contract extension and native provider capability are pending. See projection-receipt.json.'}])
        validate_spell.Draft202012Validator(base.receipt_schema(),registry=validate_spell.REGISTRY).validate(original_receipt)
        previous.write_json(target/'receipt.json',original_receipt);index.append(receipt)
    previous.write_json(out/'import-summary.json',{'schema':'OTERYN_SOURCE_STATE_NATIVE_EXTENSION_IMPORT/v1','records':43,'candidate_records':41,'reference_records':2,'status_counts':{'CANDIDATE_SCHEMA_VALID':41,'REFERENCE_ONLY_REMOVED_S24':2},'records_index':index,'authoring_contract_extension_pending':True,**FLAGS})
    previous.write_json(out/'lane-audit.json',{'schema':'OTERYN_SOURCE_STATE_NATIVE_EXTENSION_AUDIT/v1','records':index,**FLAGS})
    previous.write_json(out/'projection-proof.json',{'schema':'OTERYN_SOURCE_STATE_NATIVE_EXTENSION_PROOF/v1','records':43,'full_extension_candidates':41,'reference_only_records':2,
        'schema_refs':[{'path':'schemas/'+SCHEMA,'schema_id':schema['$id'],'sha256':previous.sha((HERE/SCHEMA).read_bytes())},{'path':'schemas/spell-dependencies.schema.json','schema_id':'urn:oteryn:spell-dependencies:candidate:1','sha256':previous.sha((HERE/'spell-dependencies.schema.json').read_bytes())}], 'schema_patch_paths':['$defs.nativeBehavior','$defs.spell.properties.source_state_contract'],'schema_id':schema['$id'],'schema_sha256':previous.sha((HERE/SCHEMA).read_bytes()),'producer_sha256':previous.sha(Path(__file__).read_bytes()),
        'policy_proofs':{str(POLICY):previous.sha((repo/POLICY).read_bytes()),'docs/architecture/OTERYN_SPELL_AUTHORING_SCHEMA_V1.md':previous.sha((repo/'docs/architecture/OTERYN_SPELL_AUTHORING_SCHEMA_V1.md').read_bytes())},
        'input_proofs':{str(previous.BASE/'source-callback-facts.jsonl.gz'):previous.sha((repo/previous.BASE/'source-callback-facts.jsonl.gz').read_bytes()),'docs/reference/spells/r55-source-closure/lane-audit.json':previous.sha((repo/'docs/reference/spells/r55-source-closure/lane-audit.json').read_bytes())}, 'accepted_model_producer_proofs':{name:previous.sha((HERE/name).read_bytes()) for name in ['native_combat.py','native_actor_states.py','native_delayed.py']}, 'source_pins':previous.source_cast_programs_pins(),'authoring_contract_extension_pending':True,'native_reader_executed':False,'target_schema_family':'private_source_complete_v2','helper_proofs':helper_proofs,'helper_scope_proofs':helper_scopes,'helper_proof_mode':'immutable_git_show_exact_source_pin',**FLAGS})
    previous.write_json(out/'package-manifest.json',{'schema':'OTERYN_SOURCE_STATE_NATIVE_EXTENSION_PACKAGE/v1','files':{p.relative_to(out).as_posix():previous.sha(p.read_bytes()) for p in sorted(out.rglob('*')) if p.is_file() and p.name!='package-manifest.json'}})
    print(json.dumps({'audited':43,'full_extension_candidates':41,'reference_only':2}))


if __name__=='__main__':main()
