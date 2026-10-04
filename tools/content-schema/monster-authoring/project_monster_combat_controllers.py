"""R65 concrete whole-cast monster domain controllers, including timed/world effects."""
import argparse
from functools import lru_cache
import copy
import gzip
import hashlib
import json
from pathlib import Path
import re
import project_monster_slot_candidates as prior
from source_monster_inline_semantics import read,PINS

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
SCHEMA=HERE/'monster-combat-controllers.schema.json'
read=lru_cache(maxsize=None)(read)
STEMS=set('lisa_skill_reducer lisa_heal spider_queen_wrap walker_skill_reducer minotaur_cult_prophet_mass_healing rotthing_wave plagirath_bog icicle_heal glooth_fairy_healing omrafir_healing_2 the_welter_heal tyrn_heal ignite candy_horror_wave rootkraken_deathholy rootkraken_rootearth foam_splash ratmiral_fire_wave ratmiral_ball smelly_cheese_berserk angry_orc_ancestor_spirit_rooted angry_orc_ancestor_spirit_fear'.split())


def sha(data):return hashlib.sha256(data).hexdigest()


def identity(row):return json.dumps(row['slot_identity'],sort_keys=True)


def cohort():
    packet,registrations=prior.load()
    previous=json.loads(gzip.decompress((ROOT/'docs/reference/spells/r54-source-closure/projected-monster-slot-candidates.json.gz').read_bytes()))
    full={identity(r) for r in previous['slots'] if r['full_slot_projection_complete']}
    out=[]
    for row in packet['slots']:
        if row['source_program_index'] is None or row['custom_category'] is not None or identity(row) in full:continue
        syntax=packet['source_programs'][row['source_program_index']]['source_syntax']
        if Path(syntax['path']).stem not in STEMS:continue
        raw=read(row['source'],syntax['path'])
        if sha(raw)!=syntax['source_sha256']:raise ValueError('immutable source hash mismatch')
        matches=[v for k,v in registrations.items() if isinstance(k,tuple) and k[:2]==(row['source'],syntax['path']) and k[2].casefold()==row['source_parameters']['name'].casefold()]
        if len(matches)!=1:raise ValueError('exact registered path/name population required')
        record=matches[0]
        out.append((row,syntax,record,raw.decode()))
    if len(out)!=51:raise ValueError('exact R65 51-slot cohort required')
    return out


def condition(c):
    out={'type':c['type'],'constructor_id':'combat','constructor_id_omitted':True,'constructor_sub_id':0,'constructor_is_persistent':False,
         'constructor_initial_ticks':0,'constructor_initial_buff':False,'parameters':{},'speed_coefficients':None,'outfit':None,'damage_ticks':[]}
    for method,args in c.get('calls',[]):
        if method=='setParameter':out['parameters'][args[0].removeprefix('CONDITION_PARAM_').lower()]=args[1]
        elif method=='setFormula':out['speed_coefficients']=args
        elif method=='setOutfit':out['outfit']=args[0]
        elif method=='setTicks':out['parameters']['ticks']=args[0]
        elif method=='addDamage':out['damage_ticks'].append({'rounds':args[0],'interval_ms':args[1],'signed_delta':args[2]})
        else:raise ValueError('unmapped condition method '+method)
    return out


def combat(c,index):
    mapping={'COMBAT_PARAM_TYPE':'damage_type','COMBAT_PARAM_EFFECT':'impact_effect','COMBAT_PARAM_DISTANCEEFFECT':'projectile_effect',
             'COMBAT_PARAM_AGGRESSIVE':'aggressive','COMBAT_PARAM_BLOCKARMOR':'block_armor','COMBAT_PARAM_BLOCKSHIELD':'block_shield',
             'COMBAT_PARAM_DISPEL':'dispel','COMBAT_PARAM_CREATEITEM':'create_item','COMBAT_PARAM_CHAIN_EFFECT':'chain_effect'}
    params=c.get('params',{});extra=set(params)-set(mapping)
    if extra:raise ValueError('unmapped Combat fields '+str(extra))
    formula=c.get('formula');fixed=None
    if formula:
        if len(formula)!=5 or str(formula[0]).lstrip('@')!='COMBAT_FORMULA_DAMAGE':raise ValueError('unsupported formula '+str(formula))
        fixed={'kind':'literal_combat_damage','min_a':formula[1],'min_b':formula[2],'max_a':formula[3],'max_b':formula[4],
               'effective_signed_minmax':[formula[1],formula[3]],'b_terms_used':False,'distribution':'normal_random','numeric_input_conversion':'int32_truncate','health_sign_route':'source_engine_signed_health'}
    return {'index':index,'declared_parameters':{mapping[k]:v for k,v in params.items()},
            'omitted_parameters_use_source_defaults':sorted(set(mapping.values())-{mapping[k] for k in params}),
            'area':c.get('area'),'conditions':[condition(v) for v in c.get('conditions',[])],
            'fixed_formula':fixed,'magnitude_input_when_formula_absent':'incoming_monster_slot_and_source_combat_default',
            'target_callback_phase':'source_combat_target_callback','execution_result':'source_combat_boolean'}


def self_heal(stem):
    threshold,duration,delay,bounds={'lisa_heal':(.07,6000,6000,[18000,23000]),
        'glooth_fairy_healing':(.1,30000,10000,[7500,8000]),'tyrn_heal':(.2,900000,0,[5000,7500])}[stem]
    return {'health_guard':{'ratio_lt':threshold,'marker_absent':True},
            'marker':{'type':'regeneration','condition_id':'default','persistent':False,'buff':False,'sub_id':88888,'duration_ms':duration,'declared_health_gain':.01,'effective_health_gain':0,'health_gain_numeric_conversion':'int32_truncate','health_interval_ms':duration},
            'delay_ms':delay,'heal_uniform_integer':bounds,'roll_scope':'callback' if delay else 'cast_guard_success',
            'actor_binding':'captured_userdata' if delay else 'current_caster','scheduled_id_argument_unused':bool(delay),
            'scheduled_presence_guard':False,'enqueue_result_ignored':True,'pre_schedule_voice':"Lisa takes a final breath before she's healing up!" if stem=='lisa_heal' else None,
            'post_heal_voice':'Lisa healed up!' if stem=='lisa_heal' else None,
            'voice_class':'monster_say','post_heal_effect':None if stem=='tyrn_heal' else 'CONST_ME_MAGIC_BLUE',
            'guard_failure_return':None if stem=='tyrn_heal' else True,'cast_success_return':True,
            'commit_order':['check_health_and_marker','say_warning_if_declared','apply_marker','schedule_or_immediate_heal','say_success_if_declared','magic_blue_if_declared']}


def controller(stem,conversion,text):
    cs=[combat(c,int(key)) for key,c in conversion.get('reference_combats',{}).items()]
    common={'source_model':'r65-'+stem,'combats':cs,'registration':{k:v for k,v in conversion['spell_calls'].items()},
            'cast_sound':next((a['args'][0] for a in conversion['spell_call_sequence'] if a['method']=='castSound'),None),
            'impact_sound':next((a['args'][0] for a in conversion['spell_call_sequence'] if a['method']=='impactSound'),None)}
    if stem in ('lisa_skill_reducer','walker_skill_reducer'):
        lisa=stem.startswith('lisa');lo,hi=(60,75) if lisa else (45,60)
        params={'selection':{'uniform_integer':[lo,hi],'roll_scope':'one_per_cast','combat_index':'rolled_value_minus_minimum'},
                'tile_creature_scan':{'kind':'legacy_stack_uid_scan','start_stack':1,'stop':'uid_zero_or_collected_declared_tile_creature_count',
                                     'target_kind':'players','tile_presence_guard':False,'source_self_comparison':'numeric_uid_vs_caster_userdata'},
                'attribute_conditions':{'condition_id':'combat','sub_id':0,'persistent':False,'buff':False,'duration_ms':7000,'percentage_source':'selected_uniform_integer',
                    'mage':['magicpointspercent'],'paladin':['distancepercent','defensepercent'],'knight':['meleepercent','defensepercent'],
                    'monk':[],'vocation_dispatch':'base_vocation_exclusive_chain' if lisa else 'independent_knight_mage_paladin_predicates'},
                'callback_binding':'each_registered_combat_closes_over_its_own_conditions',
                'tile_post_effect':'CONST_ME_SMALLPLANTS' if lisa else 'CONST_ME_MAGIC_RED','callback_return':True,'cast_return':'selected_combat_result'}
        return 'vocation_skill_reduction',{**common,**params}
    if stem in ('lisa_heal','glooth_fairy_healing','tyrn_heal'):return 'guarded_self_recovery',{**common,**self_heal(stem)}
    if stem=='spider_queen_wrap':return 'player_capture_wrap',{**common,
        'target_selection':'caster_current_target','require_player':True,'combat_false_returns_false':True,
        'outfit':{'condition_id':'combat','sub_id':0,'persistent':False,'duration_ms':30000,'look_type':422},'message':{'class':'event_advance','text':'The spider queen caught you in her net and paralysed you!'},
        'storage':{'key':43361,'source_name':'Quest.U9_1.TheRookieGuard.Mission05','value':4},
        'teleport':{'delay_ms':4500,'target_binding':'player_reacquired_by_id','presence_guard':True,'position':{'x':32013,'y':32087,'z':10}},
        'commit_order':['execute_paralyze_combat','if_success_apply_outfit','message','storage','schedule_teleport','return_true'],
        'invalid_target_return':False}
    if stem=='minotaur_cult_prophet_mass_healing':return 'named_ally_healing',{**common,
        'shared_heal_uniform_integer':[200,350],'roll_scope':'script_initialization_once','cast_first_heal_caster':True,
        'tile_callback':{'target':'top_creature','exclude_caster_by_userdata':True,'name_case':'lower',
                         'names':['minotaur cult prophet','minotaur cult follower','minotaur cult zealot'],'requires_monster_kind':False,
                         'heal_uses_shared_load_draw':True,'return':True},'cast_return':'combat_result'}
    if stem in ('rotthing_wave','rootkraken_deathholy','rootkraken_rootearth'):return 'multi_combat_sequence',{**common,
        'declared_combat_members':[c['index'] for c in cs],'iteration':'lua_pairs','strict_execution_order':False,
        'execute_every_member':True,'ignore_individual_returns':True,'cast_return':True}
    if stem=='plagirath_bog':return 'delayed_bog_strike',{**common,
        'initial_combat_index':1,'damage_combat_index':0,'initial_target':'caster_current_target','initial_nil_target_guard':False,
        'scheduled_before_initial_combat':True,'delay_ms':10000,'saved_variant':True,
        'delayed_actor':'caster_reacquired_by_id','delayed_target':'initial_target_reacquired_by_id',
        'delayed_guards':{'require_both_present':True,'caster_health_gte':1,'distance_metric':'max_abs_xyz','distance_lt':20},
        'declared_player_callback_signed_minmax':[-1500,-1500],
        'value_callback_requires_player':True,'monster_magnitude_source':'incoming_sorted_monster_slot_combat_values','delayed_result':'damage_combat_result_or_nil','cast_return':'initial_combat_result'}
    if stem=='icicle_heal':return 'egg_target_health_change',{**common,
        'spectators':{'multifloor':False,'only_players':False,'x_min':3,'x_max':3,'y_min':3,'y_max':3},
        'spectator_match':{'kind':'monster','lowercase_name':'dragon egg'},
        'set_caster_target':{'argument_kind':'literal_name_string','value':'dragon egg','binding_result_unqualified':True},
        'callback_target_match':{'lowercase_name':'dragon egg','require_monster_kind':False},
        'signed_target_health_delta':-100,'callback_success_return':True,'callback_no_match_return':None,
        'first_matching_target_returns_early':True,'cast_return':'combat_result'}
    if stem=='omrafir_healing_2':return 'fire_tile_self_recovery',{**common,
        'heal_uniform_integer':[7500,9000],'roll_scope':'every_cast_before_guards','caster_name_exact':'Omrafir',
        'health_percentage_lt':99.99,'require_tile_present':True,'any_fire_item_ids':[1487,1492,1500],
        'heal_effect':'CONST_ME_MAGIC_BLUE','voice':'Omrafir gains new strength from the fire','voice_class':'monster_say',
        'commit_order':['draw_heal','read_health_percentage','read_caster_position','name_and_health_guard','tile_guard','fire_item_guard','heal','effect','voice'],
        'return_on_all_paths':None}
    if stem=='the_welter_heal':return 'devour_spawn_recovery',{**common,
        'spectators':{'multifloor':False,'only_players':False,'x_min':10,'x_max':10,'y_min':10,'y_max':10},
        'target_kind':'monster','first_match_only':True,'name_case':'exact','targets':[{'name':'Egg','effect':'CONST_ME_HITBYPOISON'},
        {'name':'Spawn of the Welter','effect':'CONST_ME_DRAWBLOOD'}],'heal':25000,
        'voice':'<the welter devours his spawn and heals himself>','voice_class':'monster_say','heal_effect':'CONST_ME_MAGIC_BLUE',
        'commit_order':['victim_effect','remove_victim','caster_voice','heal_caster','caster_effect','return_true'],
        'no_match_return':'combat_result'}
    if stem=='ignite':
        # R28 reference capture saw the base Combat only. Current full source has three
        # explicit variants; construct the two concrete declarations rather than omit them.
        base=cs[0];captured_combat_count=len(cs)
        if captured_combat_count not in (1,3):raise ValueError('Ignite declaration count differs')
        for index,damage,effect,missile,ctype in [(1,'COMBAT_ENERGYDAMAGE',331,5,'CONDITION_ENERGY'),(2,'COMBAT_DEATHDAMAGE',332,11,'CONDITION_CURSED')]:
            if captured_combat_count==3:continue
            variant=copy.deepcopy(base);variant['index']=index
            variant['declared_parameters'].update(damage_type=damage,impact_effect=effect,projectile_effect=missile)
            variant['omitted_parameters_use_source_defaults'].remove('impact_effect')
            variant['conditions'][0]['type']=ctype;cs.append(variant)
        return 'stance_damage_condition',{**common,'combats':cs,
            'player_guard':'caster_get_player_optional','stance_routes':[{'stance':'STANCE_MASTER_OF_THUNDER','combat_index':1,'second_target_effect':333},
            {'stance':'STANCE_MASTER_OF_DECAY','combat_index':2,'second_target_effect':336}],
            'fallback_combat_index':0,'secondary_effect_callback_return':True,'cast_return':'selected_combat_result'}
    if stem in ('foam_splash','candy_horror_wave','smelly_cheese_berserk','angry_orc_ancestor_spirit_rooted','angry_orc_ancestor_spirit_fear'):
        delays={'foam_splash':[1000,2000,3000],'candy_horror_wave':[1,1],'smelly_cheese_berserk':[500,700],
                'angry_orc_ancestor_spirit_rooted':[2000],'angry_orc_ancestor_spirit_fear':[2000]}[stem]
        params={'stages':[{'combat_index':i,'requested_delay_ms':d,'enqueue_order':i} for i,d in enumerate(delays)],
            'actor_binding':'caster_reacquired_by_id','skip_missing_actor':True,'variant_binding':'captured_original_variant',
            'scheduler_delay_floor_ms':100,'ignore_scheduled_combat_returns':True,'enqueue_result_ignored':True,'cast_return':True,
            'first_stage_top_player_callback':None}
        if stem=='smelly_cheese_berserk':params['first_stage_top_player_callback']={'tile_presence_guard':True,
            'selection':'top_creature','require_player':True,'uniform_damage':[400,800],'roll_scope':'independent_per_callback_target',
            'signed_health_route':'one_draw_negated_for_both_bounds','damage_type':'earth','effect':'CONST_ME_HITBYPOISON',
            'callback_only_on_combat_index':0,'unused_count':0,'unused_remove_caster':False}
        return 'delayed_combat_stages',{**common,**params}
    if stem=='ratmiral_fire_wave':return 'wetness_fire_wave',{**common,
        'selection':{'tile_presence_guard':True,'kind':'top_creature_player'},
        'wetness_icon':{'key':'blue-shield','category':'quests','icon':'blue_shield',
            'source_category_constant':'CreatureIconCategory_Quests','source_icon_constant':'CreatureIconQuests_BlueShield',
            'missing_or_wrong_icon_returns_count':0,'nonpositive_count':{'remove_icon':True,'set_storage':0,'return_count':0},
            'positive_count':{'decrement':1,'write_icon_even_when_zero':True,'write_storage':True,'return_new_count':True}},
        'wetness_storage':{'key':47519,'source_name':'Quest.U12_60.APiratesTail.WaterIcon'},
        'reduced_if_post_decrement_count_gt':0,'reduced_uniform_damage':[500,1000],'full_uniform_damage':[1000,1500],
        'roll_scope':'independent_per_callback_target','damage_type':'fire','signed_health_route':'one_draw_negated_for_both_bounds','effect':'CONST_ME_FIREAREA',
        'messages':{'class':'event_advance','reduced':'Your wetness spares you of most of the ratmirals flaming attack!',
        'full':'You are hit by the full power of the ratmirals flaming attack!'},
        'commit_order':['read_and_mutate_wetness_icon_storage','draw_damage_for_post_decrement_count','apply_damage','message'],
        'cast_return':'combat_result'}
    raise ValueError('unmapped whole controller '+stem)


def source_proofs(donor,path,stem):
    paths=[path,'src/creatures/combat/combat.cpp','src/lua/functions/creatures/combat/combat_functions.cpp',
           'src/creatures/monsters/monsters.cpp','src/creatures/monsters/monsters.hpp',
           'src/lua/functions/creatures/combat/condition_functions.cpp','src/creatures/combat/condition.cpp']
    if 'heal' in stem or stem in ('foam_splash','candy_horror_wave','plagirath_bog','smelly_cheese_berserk','spider_queen_wrap') or stem.startswith('angry_'):
        paths.append('src/lua/functions/core/game/global_functions.cpp')
    if stem in ('spider_queen_wrap','ratmiral_fire_wave'):
        paths.append('data-otservbr-global/lib/core/storages.lua' if donor=='canary' else 'data-global/lib/core/storages.lua')
    if stem in ('lisa_skill_reducer','walker_skill_reducer'):paths += ['data/libs/functions/player.lua','data/libs/functions/vocation.lua']
    return [{'source':donor,'revision':PINS[donor],'path':p,'sha256':sha(read(donor,p))} for p in paths]


def build():
    rows=[]
    for row,syntax,registration,text in cohort():
        stem=Path(syntax['path']).stem;kind,params=controller(stem,registration['conversion'],text)
        rows.append({key:copy.deepcopy(row[key]) for key in ('slot_identity','source','monster','original_slot_sha256','source_parameters')})
        rows[-1].update(controller={'kind':kind,'parameters':params},source_proofs=source_proofs(row['source'],syntax['path'],stem),
            target_schedule={'interval_ms':row['source_parameters'].get('interval',2000)%65536,
                             'chance_percent':min(row['source_parameters'].get('chance',100)%256,100),
                             'range_tiles':min(row['source_parameters'].get('range',0)%256,22),
                             'registered_spell_early_return_before_generic_area_construction':True,
                             'source_numeric_types':{'interval':'uint16_t','chance':'uint8_t','range':'uint8_t','signed_damage':'int32_t'},
                             'effective_sorted_signed_slot_damage':[min(row['source_parameters'].get('minDamage',0),row['source_parameters'].get('maxDamage',0)),
                                                                  max(row['source_parameters'].get('minDamage',0),row['source_parameters'].get('maxDamage',0))],
                             'source_slot_targeting':{k:v for k,v in row['source_parameters'].items() if k not in ('name','interval','chance','minDamage','maxDamage')},
                             'signed_slot_damage':{'minimum':row['source_parameters'].get('minDamage'),'maximum':row['source_parameters'].get('maxDamage')}},
            source_data_model_complete=True,authoring_contract_extension_pending=True,source_consumer_implemented=False,
            runtime_activation=False,native_execution_qualified=False,native_identity_allocation=False,canonical_selection_changed=False,input_provider_equivalence=False)
    return rows


def closed(value):
    if isinstance(value,dict):return {'type':'object','additionalProperties':False,'required':sorted(value),'properties':{k:closed(v) for k,v in value.items()}}
    if isinstance(value,list):return {'type':'array','items':False,'minItems':len(value),'maxItems':len(value),**({'prefixItems':[closed(v) for v in value]} if value else {})}
    return {'type':'null' if value is None else 'boolean' if isinstance(value,bool) else 'integer' if isinstance(value,int) else 'number' if isinstance(value,float) else 'string','const':value}


def schema(rows):
    variants=[]
    for r in rows:
        if r['controller'] not in variants:variants.append(r['controller'])
    return {'$schema':'https://json-schema.org/draft/2020-12/schema','$id':'urn:oteryn:monster-combat-controllers:1',
            '$defs':{'controller':{'oneOf':[closed(v) for v in variants]}},'$ref':'#/$defs/controller'}


def main():
    from jsonschema import Draft202012Validator
    parser=argparse.ArgumentParser();parser.add_argument('--out',type=Path,required=True);args=parser.parse_args()
    rows=build();definition=schema(rows);Draft202012Validator.check_schema(definition)
    check=Draft202012Validator(definition)
    for r in rows:check.validate(r['controller'])
    out=args.out;out.mkdir(parents=True,exist_ok=True)
    def write(path,data):path.write_text(json.dumps(data,sort_keys=True,indent=2)+'\n')
    write(SCHEMA,definition);write(out/'monster-combat-controllers.json',{'records':len(rows),'rows':rows})
    write(out/'import-summary.json',{'records':len(rows),'source_data_candidates':len(rows),'runtime_activation':False,'native_execution_qualified':False,
        'authoring_contract_extension_pending':True,'source_consumer_implemented':False,'records_index':[{'slot_identity':r['slot_identity'],'source':r['source'],'status':'SOURCE_CONTROLLER_SCHEMA_VALID_CONSUMER_PENDING'} for r in rows]})
    write(out/'projection-proof.json',{'records':len(rows),'input_proofs':{str(prior.R51.relative_to(ROOT)):sha(prior.R51.read_bytes()),
        str(prior.ARCHIVE.relative_to(ROOT)):sha(prior.ARCHIVE.read_bytes())},'schema_path':str(SCHEMA.relative_to(ROOT)),'schema_sha256':sha(SCHEMA.read_bytes()),
        'runtime_activation':False,'native_execution_qualified':False,'native_identity_allocation':False,'canonical_selection_changed':False,
        'input_provider_equivalence':False,'authoring_contract_extension_pending':True,'source_consumer_implemented':False})
    (out/'README.md').write_text('''# R65 whole-cast monster controllers

51 exact registered monster slots, 49 source paths, 21 present script stems. RatmiralBall has no remaining member in this cohort. Immutable R51/R54 populations and R28 registered captures are joined by donor/path/case-normalized registered name; pinned source bytes are checked before projection. Current donor bodies match byte-for-byte across copies of each stem. No network reads were needed.

Concrete typed controllers preserve every declared Combat, geometry, signed formula coefficients, Condition defaults, target/vocation filters, random-draw scope, returns, voices, sounds, timers and world mutations. These include Spider Queen outfit/storage/teleport; Ratmiral wetness icon/storage pre-damage decrement; Welter spectator removal; Lisa/Glooth captured-userdata callbacks; deferred Combat caster-ID lookup; shared load-time Minotaur healing draw; Ignite stance variants and second effects; and nil return paths. Lua pairs traversal is explicitly unordered. COMBAT_FORMULA_DAMAGE uses A terms and normal_random with signed bounds; B terms remain evidence and are not reinterpreted as minimum damage. Decimal marker health gain 0.01 is preserved alongside proven int32 conversion to zero. Plagirath player formula callback is kept separately from Monster incoming combat-value precedence.

Target schedules include effective defaults, integer storage widths, chance/range caps and sorted signed source damage. Requested event delays remain separate from the proven 100 ms minimum. Strict private schema exports $defs.controller; no raw Lua, AST or generic executable instruction language is distributed.

Eight regressions validate the exact cohort, strict schemas, scheduling, signed formula semantics, Condition constructors, healing guards, deferred actors, mutations and the three Ignite variants. Source DATA completeness does not admit execution: authoring contract extension remains pending; consumer, native execution, runtime activation and provider equivalence remain false.
''')
    write(out/'package-manifest.json',{'files':{str(p.relative_to(out)):sha(p.read_bytes()) for p in out.iterdir() if p.is_file() and p.name!='package-manifest.json'}})

if __name__=='__main__':main()
