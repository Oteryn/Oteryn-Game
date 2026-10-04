"""R62 source-filled concrete world/control native authoring DATA, never runtime.

Every domain controller is specific, closed and source-bound. Mathematical
expressions are bounded formula data, not Lua/AST or generic executable programs.
"""
import argparse
import copy
import gzip
import hashlib
import json
import os
import re
from pathlib import Path
from functools import lru_cache

import import_source_player_bundles as base
import project_remaining_control_candidates as old
import compose_source_complete_schema as compose
import validate_spell
from jsonschema import Draft202012Validator
from referencing import Registry, Resource

HERE=Path(__file__).resolve().parent
OUT=Path('docs/reference/spells/r62-source-closure')
REV='source-world-control-r62'
SCHEMA='source-world-control-native-extensions.schema.json'
URI='urn:oteryn:source-world-control-native-extensions:1'
CACHE=Path('/workspace/spell-source-closure/generated-monsters-r28-complete/all-registered-spells.json')
SOURCE=Path('/workspace/spells-r22-monster-import-current/source-inputs')
FLAGS={k:False for k in ['runtime_activation','native_execution_qualified','canonical_selection_changed','native_identity_allocation','input_provider_equivalence','source_consumer_implemented','source_consumer_equivalence']}
PINS={'canary':'04b83b512114bfd888000d6e1433ed8ecaec7c5b','crystal':'00ce02a57ca5a12e48f32a3476e37471167e4c3f'}

def sha(b):return hashlib.sha256(b).hexdigest()
def encoded(v):return (json.dumps(v,sort_keys=True,indent=2,ensure_ascii=False)+'\n').encode()
def put(path,v):path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(encoded(v))
def folder(reg):return reg.split('/')[0]+'/'+sha(reg.encode())[:16]
def C(v):return {'const':str(v)}
def V(v):return {'input':v}
def M(op,*args):return {'operator':op,'arguments':list(args)}
def affine(level,ml,add=0):return M('add',level,M('multiply',V('magic_level'),C(ml)),C(add))
def bounds(lo,hi):return {'minimum':lo,'maximum':hi,'evaluation':'lua_Number_then_source_LuaCombat_binding_int32','sampling':'source_normal_random_inclusive','pair_semantics':'raw_source_signed_callback_pair_no_absolute_value_projection'}

@lru_cache(None)
def src(donor,path):
 os.environ['GIT_NO_LAZY_FETCH']='1'
 return base.source_file(Path('/workspace/spell-sources')/donor,PINS[donor],path)

def proof(donor,path,symbol=None,meaning=None):
 raw=src(donor,path); result={'donor':donor,'revision':PINS[donor],'path':path,'sha256':sha(raw)}
 if symbol:
  if symbol=='Spell::postCastSpell':
   match=re.search(r'(?ms)^void Spell::postCastSpell\([^\n]*bool finishedCast[^\n]*\{.*?^}',raw.decode())
   if not match:raise ValueError('finishedCast overload missing')
   body=match.group(0).encode();result['overload']='bool finishedCast, bool payCost'
  else:body=base.source_cpp_function(raw,symbol)
  result.update(symbol=symbol,body_sha256=sha(body))
 if meaning:result['semantic_binding']=meaning
 return result

def lua_proof(donor,path,symbol,meaning=None):
 raw=src(donor,path);text=raw.decode();hit=re.search(r'(?ms)^(?:local )?function '+re.escape(symbol)+r'\([^\n]*\)\n.*?^end[ \t]*$',text)
 if not hit:raise ValueError('Lua helper scope missing '+symbol)
 result=proof(donor,path,meaning=meaning);result.update(symbol=symbol,body_sha256=sha(hit.group(0).encode()),line_start=text[:hit.start()].count('\n')+1)
 return result

def flat_helper(donor):
 if donor=='crystal':
  raw=src(donor,'data/scripts/lib/register_spells.lua').decode()
  hit=re.search(r'function calculateBaseDamageHealing\(level\)\s+local step = ([^\n]+)\s+return ([^\n]+)\s+end',raw)
  if not hit:raise ValueError('Crystal base helper missing')
  # Crystal deliberately uses its accepted level curve, not Canary defect.
  return {'kind':'crystal_base_damage_healing','step':M('floor',M('divide',M('add',M('sqrt',M('add',M('multiply',C(2),V('level')),C(2025))),C(5)),C(10))),'result':M('subtract',M('add',M('floor',M('divide',M('add',V('level'),C(1000)),V('step'))),M('multiply',C(50),V('step'))),C(450)),'input':'level','numeric_type':'Lua_Number','proof':lua_proof(donor,'data/scripts/lib/register_spells.lua','calculateBaseDamageHealing','exact typed step/result')}
 return {'kind':'canary_flat_damage_healing','input':'level','level_type':'uint32','aggregated_baseline_initial':0,'current_level_baseline_initial':0,'factor_denominator_initial':5,'threshold_initial':500,'threshold_step_initial':600,'tier_initial':1,'threshold_test':'level_greater_or_equal','tier_aggregation':'add_threshold_divided_by_5_plus_tier_minus_1','factor_denominator':'5_plus_tier','next_threshold':'threshold_plus_threshold_step','next_threshold_step_add':100,'final_equation':'ceil(aggregate+(level-current_baseline)/(5+tier-1))','result_ceiling':65535,'result_type':'uint16','proof':proof(donor,'src/creatures/players/player.cpp','Player::calculateFlatDamageHealing')}

def harmony_helper():
 return {'kind':'canary_harmony_pair','inputs':['harmony','virtue','condition_serene','wheel_ascetic_stage','buff_harmonybonus'],'harmony_zero_multiplier':1,'base_percent':8,'harmony_virtue_bonus_serene':8,'harmony_virtue_bonus_other':4,'add_wheel_ascetic_stage':True,'nonzero_buff_add':'buff_minus_100','nonpositive_base_multiplier':1,'multiplier_equation':'1 + base_percent * 2^(harmony-1) / 100','output_pair_conversion':'uint64_truncate_each_nonnegative_product','proofs':[proof('canary','src/creatures/players/player.cpp','Player::getHarmonyBonus'),proof('canary','src/creatures/players/player.cpp','Player::getHarmonyDamage')]}

def target_formula(source):
 value=copy.deepcopy(source)
 def normalize(node):
  if isinstance(node,dict):
   if node.get('helper')=='canary_flat_damage_healing':node['helper']='crystal_base_damage_healing'
   for child in node.values():normalize(child)
  elif isinstance(node,list):
   for child in node:normalize(child)
 normalize(value)
 value['minimum']=M('negate',value['minimum']);value['maximum']=M('negate',value['maximum'])
 value['pair_semantics']='S27_offensive_signed_negative_target_pair_S5_level_curve';value['bounds_evaluation_stage']='before_optional_Harmony_uint64_absolute_pair_then_negative_health_projection'
 return value


def spender_lifecycle():
 return {'only_after_finished_cast':True,'order':['read_harmony_count','spend_harmony','clear_spender_cooldowns_by_2000ms_times_original_count','apply_sanctuary','ordinary_cooldowns_aggression_sounds_costs'],
  'spend_harmony':{'heal_original_count_before_reset':True,'reset_to':0,'harmony_virtue_build_one_after_reset':True,'notify_monk_data':True},
  'harmony_heal':{'party_selector':'lowest_absolute_health_in_visible_samefloor_party_getPlayers_versus_caster','removed_dead_and_caster_search_excluded':True,'distance':'client_viewport_XY','viewport_tiles':{'x':8,'y':6,'z':0},'ties':'first_strict_lower_health_candidate','origin':'ORIGIN_HARMONY','base_helper':'canary_flat_damage_healing','charge_factor_base':1,'per_charge_factor':.05,'minimum':{'coefficient':2,'floor':10,'rounding':'ceil'},'maximum':{'coefficient':2.3,'floor':25,'rounding':'ceil'},'sampling':'normal_random','aggressive':False,'impact_effect':'CONST_ME_MAGIC_BLUE'},
  'sanctuary':{'requires_wheel_instant':'SANCTUARY','original_count_must_be_nonzero':True,'skip_existing_attributes_subid':'Sanctuary','create_field':'ITEM_SANCTUARY','add_item_ignore_return':True,'start_decay':True,'duration_ms':5000,'condition_id':'CONDITIONID_DEFAULT','sub_id':'Sanctuary','damage_and_healing_dealt_percent':'100_plus_2_times_original_count'},
  'proofs':[proof('canary','src/creatures/combat/spells.cpp','Spell::postCastSpell'),proof('canary','src/creatures/combat/spells.cpp','applySanctuaryEffect'),proof('canary','src/creatures/players/player.cpp','Player::spendHarmony'),proof('canary','src/creatures/players/player.cpp','Player::healFromHarmony'),proof('canary','src/creatures/players/player.cpp','Player::buildHarmony'),proof('canary','src/creatures/combat/combat.cpp','Combat::harmonyHeal'),proof('canary','src/map/map_const.hpp')]}

def combat_data(fact,cached):
 out=[]
 for i,combat in enumerate(fact['combats']):
  reference=cached['conversion']['reference_combats'][str(i)]
  params=combat.get('parameters',{})
  out.append({'source_combat_index':i,'callback_bindings':[{'kind':c['kind'],'function':c['function']} for c in combat.get('callbacks',[])],'health_type':params.get('COMBAT_PARAM_TYPE','COMBAT_NONE'),'impact_effect':params.get('COMBAT_PARAM_EFFECT','CONST_ME_NONE'),'projectile_effect':params.get('COMBAT_PARAM_DISTANCEEFFECT','CONST_ANI_NONE'),'chain_effect':params.get('COMBAT_PARAM_CHAIN_EFFECT','CONST_ME_NONE'),'aggressive':bool(params.get('COMBAT_PARAM_AGGRESSIVE',True)),'block_armor':bool(params.get('COMBAT_PARAM_BLOCKARMOR',False)),'use_charges':bool(params.get('COMBAT_PARAM_USECHARGES',False)),'dispel_condition':params.get('COMBAT_PARAM_DISPEL'),'area':copy.deepcopy(reference.get('area')),'source_parameter_bindings':copy.deepcopy(params),'condition_declarations':copy.deepcopy(combat.get('conditions',[]))})
 return out


def challenge_helpers(donor):
 return {'target_distance':{'summons_refused':True,'reward_boss_refused':True,'distance_set':1,'duration_replaces_remaining_timer':True,'expiration':'Monster_onThink_decrements_to_zero_restores_type_target_distance_and_refreshes_icon','icon_refresh_when_base_distance_exceeds_requested':True},'challenge':{'summons_refused':True,'select_target_required':True,'focus_duration_set_on_success':True,'target_change_ticks_reset':0,'canary_battle_healing_on_success':donor=='canary','battle_healing':None if donor!='canary' else {'wheel_instant':'Battle Healing','target':'challenging_player_self','base':'shielding_skill_times_0_2','health_percent':'integer_health_times_100_divide_maxhealth','multiplier_at_percent_le_30':3,'multiplier_at_percent_le_60':2,'otherwise_multiplier':1,'amount_conversion':'int32_truncate','proofs':[proof(donor,'src/creatures/players/components/wheel/player_wheel.cpp','PlayerWheel::healIfBattleHealingActive'),proof(donor,'src/creatures/players/components/wheel/player_wheel.cpp','PlayerWheel::checkBattleHealingAmount')]},'lua_wrapper_ignores_target_result':True,'lua_default_cooldown_ms':6000},'proofs':[proof(donor,'src/creatures/monsters/monster.cpp','Monster::changeTargetDistance'),proof(donor,'src/creatures/monsters/monster.cpp','Monster::challengeCreature'),proof(donor,'src/creatures/monsters/monster.cpp','Monster::onThink'),proof(donor,'src/lua/functions/core/game/global_functions.cpp','GlobalFunctions::luaDoChallengeCreature'),proof(donor,'src/lua/functions/creatures/monster/monster_functions.cpp','MonsterFunctions::luaMonsterChangeTargetDistance')]}

def shared_selector():
 return {'required_stance':'STANCE_SHARED_CONSERVATION','player_required':True,'party_required':True,'member_sequence':'getMembers_then_append_leader','primary_target_excluded':True,'caster_excluded':True,'living_player_only':True,'same_floor':True,'distance':'Chebyshev_XY_to_caster','comparison':'strict_less_than','equal_distance':'first_eligible_in_ipairs_sequence','maximum_distance':None,'viewport_filter':False,'secondary_ratio':0.30,'secondary_variant':'numeric_creature_ID','secondary_has_paralyze_dispel':False,'secondary_result_ignored':True}

def locate_geometry():
 return {'delta':'caster_minus_target','distance':'max_abs_XY','floor_relation':'casterZ_minus_targetZ','direction_distance_min':5,'close_distance_exclusive':101,'far_distance_exclusive':275,'slope_dx_zero':10,'slope_thresholds':[0.4142,2.4142],'floor_text':['is below you','is standing next to you','is above you'],'distance_text':['is on a lower level to the','is to the','is on a higher level to the','is far to the','is very far to the'],'direction_branches':{'abs_tangent_less_than_0_4142':{'dx_positive':'west','otherwise':'east'},'abs_tangent_less_than_2_4142':{'tangent_positive':{'dy_positive':'north-west','otherwise':'south-east'},'otherwise':{'dx_positive':'south-west','otherwise':'north-east'}},'otherwise':{'dy_positive':'north','otherwise':'south'}},'direction_text':['north','south','east','west','north-east','north-west','south-east','south-west'],'floor_mapping':{'dz_positive':'higher','dz_negative':'lower','zero':'same'},'message_class':'MESSAGE_LOOK','success_effect':'CONST_ME_MAGIC_BLUE'}

def model(donor,name,raw,fact,cached):
 text=raw.decode(); common={'source_model':'r62/'+name.replace("'",'').replace(' ','_'),'source_costs_unchanged':True,'formula_math_semantics':'Lua_binary64_with_ordered_left_fold_for_nary_arithmetic','source_monk_spell_type':fact['registrar'].get('monkSpellType'),'combat_definitions':combat_data(fact,cached),'source_cast_return':'combat_result','source_sound_bindings':{k:v for k,v in fact['registrar'].items() if k in ('castSound','impactSound')},'helper_definitions':[]}
 params=common; key=None
 if name=='desintegrate_rune':
  key='tile_item_operation';params.update(tile_missing='skip_to_final_presentation',items_missing='skip_to_final_presentation',iteration='ipairs',visited_item_limit=500,removal={'movable_required':True,'unique_id_strictly_greater_than':65535,'action_id_required':0,'excluded_item_ids':[4240,4241,4242,4243,4246,4247,4248]},final_cancel='RETURNVALUE_NOTPOSSIBLE',final_effect='CONST_ME_POFF',source_cast_return='literal_true_after_final_presentation')
 elif name in ('magic_wall','wild_growth'):
  key='tile_item_operation';growth=name=='wild_growth'; prefix='ITEM_WILDGROWTH' if growth else 'ITEM_MAGICWALL'
  symbols=[prefix,prefix+'_SAFE'];enums=src(donor,'src/utils/utils_definitions.hpp').decode();items={s:int(re.search(r'\b'+s+r'\s*=\s*(\d+)',enums).group(1)) for s in symbols}
  params.update(tile_guards={'missing':'false','floor_change':'false','top_non_player_creature':'false'},safe_item_when=['IsExpertPVP','WORLD_TYPE_NO_PVP'] if donor=='canary' else ['WORLDTYPE_OPTIONAL'],item_variants=items,create_count=1,duration_seconds={'minimum':30 if growth else 16,'maximum':30 if growth else 24,'distribution':'uniform_random_inclusive' if not growth else 'constant'},item_duration={'mutation_scope':'shared_ItemType','show_duration':True,'decay_to_default':0,'start_decaying':True},description_template='Casted by: %s',expert_pvp_context_from_caster=donor=='canary',creation_position='none_then_addItemEx' if donor=='canary' else 'target_position',insert_flag='FLAG_NOLIMIT' if donor=='canary' else None,insert_error='callback_false' if donor=='canary' else 'Game_createItem_failure_nil',creation_failed='callback_nil',callback_success='implicit_nil',helper_definitions=[{'kind':'Item_duration','proof':proof(donor,'src/lua/functions/items/item_functions.cpp','ItemFunctions::luaItemSetDuration')}])
 elif name in ('challenge','chivalrous_challenge','divine_dazzle','balanced_brawl'):
  key='monster_ai_override';params['helper_definitions']=[challenge_helpers(donor)]
  if name=='challenge':params.update(target_callback='doChallengeCreature',challenge_cooldown_ms=6000,callback_return='lua_wrapper_true_when_creatures_exist')
  elif name=='balanced_brawl':
   params.update(distance=1,duration_ms=16000,target_callback='monster_change_distance' if donor=='canary' else 'all_creatures_on_each_target_tile',monster_only=True,source_callback_return='true',change_distance_result_ignored=True)
   if donor=='crystal':params.update(tile_missing='source_error_before_getCreatures',creature_iteration='pairs_ids_then_live_Creature',base_target_distance_must_exceed=1,reward_boss_viewport_refusal=True,no_ranged_refusal=True,boss_message="You can't use this spell if there's a boss.",no_target_message='There are no ranged monsters.',refusal_effect='CONST_ME_POFF',failure_return=False,source_cast_return='guarded_boolean')
  else:
   challenge=name=='chivalrous_challenge';params.update(chain={'initial_count':(5 if donor=='canary' else 6) if challenge else 3,'additional_targets':'player_WheelSpellAdditionalTarget_for_exact_spell_name','jump_radius':6 if donor=='canary' else 7,'backtracking':False,'picker':{'monster_only':True,'reward_boss_refused':True,'master_must_be_nil':True,'type_target_distance_greater_than':1}},distance=1,duration_ms=12000,additional_duration='player_WheelSpellAdditionalDuration_seconds_times_1000',challenge_cooldown_ms=12000 if challenge else None,reward_boss_viewport_refusal=True,viewport_arguments=[False,False],boss_message="You can't use this spell if there's a boss.",no_target_message='There are no ranged monsters.',refusal_effect='CONST_ME_POFF',target_callback_returns_true=True,source_cast_return='guarded_boolean',challenge_uses_extended_duration=False)
 elif name in ('find_person','find_fiend'):
  key='locate_message';params.update(geometry=locate_geometry(),source_cast_return='guarded_boolean',failure_effect='CONST_ME_POFF')
  if name=='find_person':params.update(target_lookup='Player_variant_string',access_guard='target_access_and_not_caster_access',failure='RETURNVALUE_PLAYERWITHTHISNAMEISNOTONLINE',success_message_template='{target_name} {location}.')
  else:params.update(target_lookup='ForgeMonster_pickClosestFiendish_then_Creature_ID',failure='No creatures around',difficulty={'requires_type_and_unlocked_bestiary':True,'kill_amount_provider':'monsterType_BestiarytoKill_not_player_current_kills','kill_bands':[[5,25,'Harmless'],[None,250,'Trivial'],[None,500,'Easy'],[None,1000,'Medium'],[None,2500,'Hard'],[None,5000,'Challenging']],'otherwise':'Unknown','zero_to_four_maps_to':'Trivial'},picker={'candidate_list':'Game_getFiendishMonsters','iteration':'pairs','live_monster_required':True,'distance':'Chebyshev_XYZ','sort_comparator':'strict_less_than','equal_distance_order':'unspecified_Lua_table_sort','missing_player_returns':0,'empty_candidates_returns':False},time_left={'append_when_floor_minutes_less_than':15,'deadline':'monster_getTimeToChangeFiendish','clock':'os_time_seconds','negative_seconds_clamped':False,'whole_minutes_floor':True,'append_seconds_when_less_than':60,'prefix':'This monster will stay fiendish for less than','minutes_template':' {floor_minutes} minutes and','seconds_template':' {seconds} seconds.'},success_message_template='The monster {location}. Be prepared to find a creature of difficulty level "{difficulty}".',helper_definitions=[{'kind':'ForgeMonster_closest_and_time','proofs':[lua_proof(donor,'data/libs/systems/exaltation_forge.lua',name) for name in ['ForgeMonster:pickClosestFiendish','ForgeMonster:getTimeLeftToChangeMonster','getFiendishMinutesLeft']],'distance_proof':proof(donor,'src/lua/functions/map/position_functions.cpp','PositionFunctions::luaPositionGetDistance')}])
 elif name in ('blood_rage','protector'):
  key='stance_toggle'; blood=name=='blood_rage';params.update(condition={'type':'CONDITION_ATTRIBUTES','id':'CONDITIONID_COMBAT','sub_id':'AttrSubId_BloodRageProtector','duration_ms':10000 if blood else 13000,'buff_spell':True,'modifiers':{'melee_skill_percent':135,'damage_received_percent':115,'disable_defense':True} if blood else {'shield_skill_percent':220,'damage_dealt_percent':65,'damage_received_percent':85}},existing_same_condition_removed_before_combat=True,removal_result_ignored=True,combat_failure_does_not_restore_previous_condition=True)
 elif name=='mentor_other':
  key='mentor_other';params.update(target_player_required=True,base_vocation_zero_refused=True,condition_type='CONDITION_MENTOROTHER',condition_sub_id='MentorOther',duration_ms=60000,base_vocation_modifiers=[{'base_vocation':1,'damage_dealt_percent':105},{'base_vocation':2,'healing_dealt_percent':105},{'base_vocation':3,'auto_attack_dealt_percent':105},{'base_vocation':4,'damage_received_percent':97},{'base_vocation':9,'harmony_bonus_percent':102}],unmapped_vocation='callback_false',add_condition_result_ignored=True,matched_vocation_callback_return=True)
 elif name in ('chained_penance','devastating_knockout','greater_tiger_clash','tiger_clash','sweeping_takedown'):
  key='equipment_attack';params['source_health_delta_route']='positive_callback_value_heals_even_COMBAT_PHYSICALDAMAGE';params['signed_health_proofs']=[proof(donor,'src/creatures/combat/combat.cpp','ValueCallback::getMinMaxValues'),proof(donor,'src/game/game.cpp','Game::combatChangeHealth')];params['helper_definitions']=[flat_helper(donor)];params['input_bindings']={'attack_skill':'ValueCallback_skill_weapon_calculateSkillFormula','attack_value':'ValueCallback_attack_weapon_calculateSkillFormula','level':'Player_member_level','attack_factor':'source_unused','no_weapon':[0,7,0],'callback_to_engine':'Lua_Number_to_int32_then_normal_random'}
  power={'chained_penance':74,'devastating_knockout':62,'greater_tiger_clash':44,'tiger_clash':15,'sweeping_takedown':48}[name]
  basevalue=M('add',M('multiply',C(power),M('divide',V('attack_skill'),C(100)),M('divide',V('attack_value'),C(10))),{'helper':'canary_flat_damage_healing','input':'level'})
  if name=='sweeping_takedown':
   basevalue=M('add',M('divide',M('multiply',V('attack_skill'),V('attack_value'),C(power)),C(1000)),{'helper':'canary_flat_damage_healing','input':'level'})
   bonus={'kind':'skill_quadratic_bonus','delta_threshold':110,'equation':'(attack_skill-110)^2 * coefficient','strict_thresholds_descending':[[250,.043],[230,.039],[210,.037],[190,.035],[160,.033],[140,.029],[130,.026],[120,.024],[110,.022]],'otherwise':0}
   params.update(skill_bonus=bonus,formula=bounds(M('multiply',M('add',basevalue,V('skill_quadratic_bonus')),C(1.3)),M('multiply',M('add',basevalue,V('skill_quadratic_bonus')),C(1.7))),harmony_pair_multiplier=True,outer_bound_ratio=.75,cache_key='caster_ID',cache_scope='source_file_local_shared',missing_cache_pair=[0,0],missing_cache_log='debug',outer_cache_log='trace',execution_order=['inner_execute_ignore_result','outer_execute_ignore_result','clear_cache','return_true'],exception_cleanup=False,source_cast_return='literal_true_after_both_combats')
  else:
   lo=M('subtract',basevalue,M('divide',basevalue,C(10)));hi=M('add',basevalue,M('divide',basevalue,C(10)))
   if name=='tiger_clash':lo=M('maximum',lo,C(5));hi=M('maximum',hi,C(10))
   params.update(formula=bounds(lo,hi),harmony_pair_multiplier=name!='chained_penance',clamp_before_harmony=name=='tiger_clash')
  if name!='chained_penance':params['helper_definitions'].append(harmony_helper());params['spender_lifecycle']=spender_lifecycle()
  else:params.update(chain={'initial_count':3,'additional_targets':'player_WheelSpellAdditionalTarget_Chained_Penance','jump_radius':3,'backtracking':False,'picker':{'npc_refused':True,'caster_refused':True,'protection_zone_refused':True}})
 elif name in ('heal_friend',"nature's_embrace"):
  key='shared_conservation';nature=name!="heal_friend";level=M('divide',V('level'),C(2.5)) if nature else {'helper':'crystal_base_damage_healing','input':'level'}
  lo=affine(level,22 if nature else 11,0 if nature else 4);hi=affine(level,26 if nature else 13,0 if nature else 5)
  if nature:lo=M('multiply',lo,M('divide',C(2000),C(650)));hi=M('multiply',hi,M('divide',C(2000),C(650)))
  params.update(primary_formula=bounds(M('floor',lo) if nature else lo,M('floor',hi) if nature else hi),secondary_formula=bounds(M('floor',M('multiply',lo,C(.30))) if nature else M('multiply',lo,C(.30)),M('floor',M('multiply',hi,C(.30))) if nature else M('multiply',hi,C(.30))),party_selector=shared_selector(),secondary_attempted_even_primary_false=True,execution_order=['primary_execute','secondary_helper','return_primary_result'],self_refusal=nature,caster_pre_primary_blue=not nature)
  if nature:params['self_refusal_message']="You can't cast this spell to yourself."
  else:params['helper_definitions']=[flat_helper(donor)]
 elif name=='mass_spirit_mend':
  key='mass_spirit_mend';params.update(caster_player_required=True,non_player_helper_return='nil',callback_return_ignores_helper_result=True);level={'helper':'crystal_base_damage_healing','input':'level'};params.update(formula=bounds(M('floor',affine(level,5.7,26)),M('floor',affine(level,10.43,62))),self_formula=bounds(M('floor',affine(level,12,75)),M('floor',affine(level,20,125))),rng='Lua_math_random_inclusive',random_draw_before_target_eligibility=True,self_consumes_discarded_initial_draw=True,excluded_name='specific_creature_name',target_name_casefold=True,eligible={'players':True,'leiden':True,'other_monsters':['ravenous hunger','dorokoll the mystic','eshtaba the conjurer','eliz the unyielding','mezlon the defiler','malkhar deathbringer',"azaram's soul",'containment crystal','rift fragment']},leiden_direct_health_sign='positive_heal',leiden_effect='CONST_ME_MAGIC_RED',other_effect='CONST_ME_MAGIC_BLUE',target_callback_return=True,helper_definitions=[flat_helper(donor)])
 elif name in ('death_echo','divine_grenade'):
  key='delayed_strike';echo=name=='death_echo';level={'helper':'crystal_base_damage_healing','input':'level'};params['helper_definitions']=[flat_helper(donor)]
  params.update(delay_ms=1000 if echo else 3000,delayed_requires_tile=True,delayed_requires_live_player_ID=True,delayed_variant={'type':2,'instant_name':'Death Echo' if echo else 'Divine Grenade','rune_name':'','position':'captured_cast_position'},delayed_formula_inputs='reevaluated_at_delayed_combat',source_cast_return='literal_true_after_schedule' if echo else 'guarded_boolean')
  if echo:params.update(primary_formula=bounds(M('negate',affine(level,2.4)),M('negate',affine(level,3.6))),echo_formula_ratio=.5,stance_element_selection={'STANCE_MASTER_OF_FLAMES':{'element':'fire','effect':323},'STANCE_MASTER_OF_THUNDER':{'element':'energy','effect':322},'default':{'element':'death','effect':321}},stance_captured_at_cast=True,primary_result_ignored=True,execution_order=['retune_shared_primary','execute_primary','schedule_echo','return_true'],echo_retunes_shared_combat=True)
  else:params.update(formula=bounds(M('negate',affine(level,4)),M('negate',affine(level,6))),wheel_grade_multiplier=[1,1.3,1.6,2],caster_player_required=True,tile_guards={'tile_and_ground_required':True,'solid_or_projectile_block_refused':True,'immovable_solid_refused':True,'house_refused':True,'protection_zone_refused':True,'creatures_refused':False,'floorchange_refused':False},failure_message='You cannot throw the grenade there.',failure_effect='CONST_ME_POFF',initial_indicator='CONST_ME_DIVINE_GRENADE',schedule_order=['explode_at_3000','remove_indicator_at_3000'],dormant_target_callback={'reachable_from_cast':False,'center':'caster_position_getWithinRange_target_4','creature_and_target_required':True,'caster_player_required':True,'initial_indicator':False,'same_two_scheduled_events':True},helper_definitions=[flat_helper(donor),{'kind':'Tile_isWalkable','proof':lua_proof(donor,'data/libs/functions/tile.lua','Tile:isWalkable','true PZ argument, omitted options false')}])
 else:raise ValueError('unmapped source controller '+donor+'/'+name)
 if name in ('chained_penance','devastating_knockout','greater_tiger_clash','tiger_clash','sweeping_takedown'):
  params.update(target_damage_formula=target_formula(params['formula']),target_level_helper=flat_helper('crystal'),target_health_delta_route='negative_health_delta_damage',target_pair_transform_order=['S5_Crystal_level_curve','source_skill_bounds_and_clamps','source_Harmony_uint64_pair_if_spender','negate_final_pair'],source_health_delta_equivalence=False,accepted_target_normalization=True,accepted_normalization_decisions=['S5','S27'],source_formula_is_raw_positive_heal_evidence=True)
 if name in ('magic_wall','wild_growth'):
  if donor=='canary':params['helper_definitions'].append({'kind':'expert_world_type','world_type_string_casefold':True,'expert_string':'expert-pvp','proofs':[lua_proof(donor,'data/global.lua','getWorldType'),lua_proof(donor,'data/global.lua','IsExpertPVP')]})
  if donor=='canary':params['helper_definitions'].append({'kind':'expert_pvp_barrier_context','requires_item_owner_and_expert_enabled':True,'owner_player':'caster_player_or_summon_owner','owner_snapshot_fields':['GUID','normalized_PvP_mode','targets_at_cast','attackers_at_cast','owner_was_player_or_summon'],'canonical_item':'ITEM_WILDGROWTH' if name=='wild_growth' else 'ITEM_MAGICWALL','safe_visual_item':'ITEM_WILDGROWTH_SAFE' if name=='wild_growth' else 'ITEM_MAGICWALL_SAFE','blocking_visual_item':'ITEM_WILDGROWTH' if name=='wild_growth' else 'ITEM_MAGICWALL','write_custom_attributes':True,'callback_ignores_return':True,'proofs':[proof(donor,'src/lua/functions/items/item_functions.cpp','ItemFunctions::luaItemSetExpertPvpFieldContext'),proof(donor,'src/creatures/players/components/pvp/expert_pvp.cpp','ExpertPvp::attachFieldContext'),proof(donor,'src/creatures/players/components/pvp/expert_pvp.cpp','ExpertPvp::makeFieldContext')]})
  params['helper_definitions'].append({'kind':'barrier_item_constants','proof':proof(donor,'src/utils/utils_definitions.hpp')})
 if name=='divine_grenade':params['helper_definitions'].append({'kind':'Position_getWithinRange','distance':'Chebyshev_XY','distance_greater_than_range':'return_caster_position','otherwise':'return_target_position','range':4,'proof':lua_proof(donor,'data/libs/functions/position.lua','Position.getWithinRange')})
 if key is None:raise ValueError('untyped controller')
 if donor=='crystal' and any(k in params for k in ['formula','primary_formula']):
  params['formula_input_binding']={'level':'Player_getLevel','magic_level':'Player_getMagicLevel_direct' if name=='mass_spirit_mend' else 'ValueCallback_getMagicLevelSkill','specialized_magic_level':None if name=='mass_spirit_mend' else {'input':'current_primary_combat_type','source_second_argument':True,'added_after_runic_mastery':True},'runic_mastery':None if name=='mass_spirit_mend' else {'required_wheel_instant':'Runic Mastery','instant_spell_name_must_be_empty':True,'matching_distinct_rune_and_conjuring_spells_required':True,'random_inclusive_range':[0,100],'sampler':'source_normal_random','success_less_or_equal':25,'can_cast_bonus_percent':20,'cannot_cast_bonus_percent':10,'integer_percent_increment_before_specialized_level':True},'base_power_callback_parameter':'unused','proofs':[proof(donor,'src/creatures/combat/combat.cpp','ValueCallback::getMinMaxValues'),proof(donor,'src/creatures/combat/combat.cpp','ValueCallback::getMagicLevelSkill')]}
 if donor=='canary' and name in ('chained_penance','devastating_knockout','greater_tiger_clash','tiger_clash','sweeping_takedown'):
  params['source_post_callback_binding']={'pair_conversion':'Lua_Number_to_int32','sample':'normal_random','weapon_secondary_split_when':'weapon_calculateSkillFormula_returns_true','element_ratio':'element_attack_divide_total_attack_value','secondary_amount':'round(sample_times_element_ratio)','primary_amount':'round(sample_times_1_minus_element_ratio)','otherwise_secondary_type':'COMBAT_NONE','otherwise_secondary_value':0,'proof':proof(donor,'src/creatures/combat/combat.cpp','ValueCallback::getMinMaxValues')}
 roles={'death_echo':{'primary':0,'delayed_echo':1},'divine_grenade':{'delayed_explosion':0,'unreachable_cast_controller':1},'heal_friend':{'primary':0,'secondary_shared_conservation':1},"nature's_embrace":{'primary':0,'secondary_shared_conservation':1},'sweeping_takedown':{'inner':0,'outer':1}}
 params['combat_bindings']=roles.get(name,{'primary':0} if params['combat_definitions'] else {})
 params['source_function_scopes']=[]
 for match in re.finditer(r'(?ms)^(?:local )?function ([A-Za-z_][A-Za-z0-9_.:]*)\([^\n]*\)\n.*?^end[ \t]*$',text):
  params['source_function_scopes'].append({'symbol':match.group(1),'body_sha256':sha(match.group(0).encode()),'line_start':text[:match.start()].count('\n')+1,'typed_controller_binding':name})
 return {'key':key,'parameters':params}


def closed(v):
 if isinstance(v,dict):return {'type':'object','additionalProperties':False,'required':list(v),'properties':{k:closed(x) for k,x in v.items()}}
 if isinstance(v,list) and not v:return {'type':'array','maxItems':0,'items':False}
 if isinstance(v,list):return {'type':'array','minItems':len(v),'maxItems':len(v),'prefixItems':[closed(x) for x in v],'items':False}
 return {'const':v}

def extension(rows):
 alternatives=[]
 for r in rows:
  if 'spell' in r:alternatives.append(closed(r['spell']['spell']['execution']['native_behavior']))
 return {'$schema':'https://json-schema.org/draft/2020-12/schema','$id':URI,'title':'Closed source world/control DATA extension r62; consumer pending','$defs':{'nativeBehavior':{'anyOf':alternatives}}}


def build(repo):
 repo=Path(repo);audits=json.loads((repo/'docs/reference/spells/r58-source-closure/lane-audit.json').read_bytes())['records']; selected=[r for r in audits if r['status']!='CANDIDATE_SCHEMA_VALID']
 if len(selected)!=37:raise ValueError('requires exact r58 remaining31 plus6 references')
 facts={r['registration_key']:r for r in map(json.loads,gzip.decompress((repo/old.BASE/'source-callback-facts.jsonl.gz').read_bytes()).splitlines())};caches={(r['source'],r['provenance']['path']):r for r in json.loads(CACHE.read_bytes())};rows=[]
 for audit in selected:
  reg=audit['registration_key'];f=facts[reg];donor=reg.split('-')[0];fact=f['source_callback_facts'];path=fact['file'];raw=src(donor,path);original=repo/old.BASE/folder(reg);headerraw=(original/'source-header.json').read_bytes()
  if f['source_revision']!=PINS[donor] or sha(raw)!=f['source_sha256']:raise ValueError('source pin differs '+reg)
  row={'registration_key':reg,'source_sha256':f['source_sha256'],'source_revision':f['source_revision'],'source_header_sha256':sha(headerraw),'source_header_bytes':headerraw,'baseline_lane':audit['baseline_lane'],'name':audit['name'],**FLAGS}
  if audit.get('disposition') in ('DISABLED_REFERENCE_EXAMPLE','RETIRED_REFERENCE_ONLY_S24'):
   row.update(status='REFERENCE_ONLY_DISABLED' if audit['disposition']=='DISABLED_REFERENCE_EXAMPLE' else 'REFERENCE_ONLY_RETIRED_S24',disposition=audit['disposition']);rows.append(row);continue
  cached=caches[(donor,path)]
  if cached['provenance']['sha256']!=f['source_sha256'] or not cached['conversion']['reference_capture_complete']:raise ValueError('combat cache source differs '+reg)
  identity={'key':'candidate:spell/source/'+reg.split('/')[0]+'/'+sha(reg.encode())[:16],'revision':REV};defaults,defaultproofs=base.default_fields(Path('/workspace/spell-sources')/donor,f['source_revision']);spell=base.fill_header(json.loads(headerraw)['spell'],defaults,identity,fact['registrar']);spell['requirements'].pop('vocation_display_flags',None);spell['targeting'].setdefault('allow_on_self',defaults['allowOnSelf']);spell['targeting'].setdefault('check_floor',True)
  name=Path(path).stem;native=model(donor,name,raw,fact,cached);spell['execution']={'native_behavior':native};catalog={'definitions':[]}
  if spell['carrier']=='rune':
   spell['rune']['item']['revision']=REV
   catalog['definitions'].append(spell['rune']['item'])
  row.update(status='CANDIDATE_SCHEMA_VALID',candidate_key=identity['key'],candidate_revision=REV,spell={'spell':spell},dependencies={'abilities':[],'effects':[],'formulas':[]},catalog=catalog,source_default_proofs=defaultproofs,authoring_contract_extension_pending=True,source_full_controller_data_complete=True,source_numeric_equivalence=False,target_schema_family='private_source_complete_v2',native_consumer_identity_guard='CAPABILITY_BLOCKED_SOURCE_FILLED_NEW_PRIVATE_PARAMETERS',source_helper_execution_qualified=False,source_fact_sha256=sha(old.canonical(f)))
  rows.append(row)
 return rows


def policy_proofs(repo):
 proofs=[]
 for path in ['docs/architecture/OTERYN_SPELL_AUTHORING_SCHEMA_V1.md','docs/architecture/OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md']:
  proof={'path':path,'sha256':sha((repo/path).read_bytes())}
  rows=[line for line in (repo/path).read_text().splitlines() if '| S27 |' in line or '| S24 |' in line or '| S5 |' in line]
  if rows:proof['exact_rows']=rows
  proofs.append(proof)
 return proofs


def emit(repo):
 repo=Path(repo).resolve();rows=build(repo);schema=extension(rows);put(HERE/SCHEMA,schema);validator,composed,_=compose.validator([HERE/SCHEMA],list(validate_spell.SCHEMAS.values()));out=repo/OUT
 for row in rows:
  target=out/folder(row['registration_key']);target.mkdir(parents=True,exist_ok=True);(target/'source-header.json').write_bytes(row['source_header_bytes'])
  original=json.loads((repo/old.BASE/folder(row['registration_key'])/'receipt.json').read_bytes())
  if 'spell' in row:
   errors=list(validator.iter_errors(row['spell']))
   if errors:raise ValueError(row['registration_key']+': '+str(errors[0]))
   for field in ['spell','dependencies','catalog']:put(target/(field+'.json'),row[field])
   original.update(conversion_notes=['Source-filled closed '+row['spell']['spell']['execution']['native_behavior']['key']+' controller with typed fields, original header costs, scoped helper proofs and combat bindings.', 'Private source-complete v2 DATA only; native/runtime consumer and authoring contract extension remain pending.']+(['Accepted S5/S27 offensive target uses Crystal level curve and negative health deltas; raw positive source healing formulas and equivalence=false remain separate evidence.'] if row['spell']['spell']['execution']['native_behavior']['parameters'].get('accepted_target_normalization') else []),engine_default_proofs=row['source_default_proofs'],status='CANDIDATE_SCHEMA_VALID',blockers=[],dependencies={'abilities':0,'effects':0,'formulas':0},native_execution_qualified=False,item_owner_bindings_required=row['catalog']['definitions'],schema_and_semantic_validation_errors=[],remaining_mechanics=[{'source_field':'runtime.native_and_extension_owner','reason':'Concrete source-filled controller DATA validates against private source-complete schema; accepted contract and runtime consumer remain pending.'}])
  frozen=[json.loads(p.read_bytes()) for p in (repo/'imports/spells/r28/schemas').glob('*.schema.json')];receipt_registry=Registry().with_resources((s['$id'],Resource.from_contents(s)) for s in frozen if '$id' in s);receipt_validator=Draft202012Validator(json.loads((repo/'imports/spells/r28/schemas/player-bundle-receipt.schema.json').read_bytes()),registry=receipt_registry);receipt_errors=list(receipt_validator.iter_errors(original))
  if receipt_errors:raise ValueError(row['registration_key']+': immutable r28 receipt '+str(receipt_errors[0]))
  put(target/'receipt.json',original);put(target/'projection-receipt.json',{k:v for k,v in row.items() if k not in ('source_header_bytes','spell','dependencies','catalog')})
 index=[{k:v for k,v in r.items() if k not in ('source_header_bytes','spell','dependencies','catalog')} for r in rows];full=sum('spell' in r for r in rows)
 put(out/'schemas'/SCHEMA,schema);put(out/'schemas'/'spell-source-complete.schema.json',composed);(out/'schemas'/'spell-dependencies.schema.json').write_bytes((HERE/'spell-dependencies.schema.json').read_bytes())
 put(out/'lane-audit.json',{'records':index,**FLAGS});put(out/'import-summary.json',{'schema':'OTERYN_SOURCE_WORLD_CONTROL_IMPORT/v1','records':len(rows),'candidate_records':full,'reference_records':len(rows)-full,'records_index':index,'status_counts':{status:sum(r['status']==status for r in rows) for status in sorted({r['status'] for r in rows})},'authoring_contract_extension_pending':True,**FLAGS})
 put(out/'projection-proof.json',{'schema':'OTERYN_SOURCE_WORLD_CONTROL_PROOF/v1','full_extension_candidates':full,'reference_records':len(rows)-full,'source_pins':PINS,'base_capture_path':str(old.BASE/'source-callback-facts.jsonl.gz'),'base_capture_sha256':sha((repo/old.BASE/'source-callback-facts.jsonl.gz').read_bytes()),'schema_id':URI,'schema_sha256':sha(encoded(schema)),'producer_sha256':sha(Path(__file__).read_bytes()),'schema_patch_paths':['$defs.nativeBehavior'],'authoring_contract_extension_pending':True,'native_reader_executed':False,'verification':{'command':'/workspace/spell-tools/bin/python -m unittest discover -s tools/content-schema/spell-authoring -p test_project_source_world_control_candidates.py -q','tests_passed':7,'test_source_sha256':sha((HERE/'test_project_source_world_control_candidates.py').read_bytes()),'actual_immutable_CPP_helpers':True,'actual_immutable_Lua_callbacks':True,'separate_S5_S27_target_math_grid':True,'independent_reviewer_tests_passed':7},'target_schema_family':'private_source_complete_v2','policy_proofs':policy_proofs(repo),**FLAGS})
 (out/'README.md').write_text('''# R62 — world/control source DATA candidates

31 active source registrations have source-filled typed Spell models. Four disabled
examples and two S24-retired Expose Weakness registrations remain reference-only.
Every source-header.json is byte-identical to sealed r28; source costs are retained.

The closed private native extension exposes concrete tile operations/barriers,
monster targeting/chains, location messages, attribute/mentor conditions, Monk
spenders, Shared Conservation, Mass Spirit Mend and delayed spell controllers.
Named scoped Lua/C++ hashes bind helper transforms, branches and combat indices.
No source Lua, generic AST interpreter or copied canonical native profile is used
as the target executable definition. Formula expressions are typed arithmetic data.

For five offensive Canary Monk controllers, raw positive source callback bounds
and the actual C++ healing route remain separate evidence. Accepted S5/S27 target
bounds use the Crystal level curve and negative health deltas for damage. Their
source health equivalence is explicitly false. Spender lifecycle data preserves
Harmony heal/reset/rebuild, cooldown adjustments and Sanctuary ordering.

All 31 validate with the private source-complete Spell v2 composition. Seven tests
pass, including original immutable C++ Flat/Harmony helpers, original Lua callback
formula grids, a separate normalized target grid, and Mass Spirit Mend random draw
ordering/positive Leiden healing. Independent review reran all seven tests.

CANDIDATE_SCHEMA_VALID is a DATA qualification. The authoring extension contract,
operational reader/native providers and runtime admission are pending. Every
projection receipt keeps source_consumer_implemented, runtime_activation and
native_execution_qualified false. No live server import or runtime change occurs.
All research for this packet used cached pinned Git objects and local source
captures; no web or Remote Desktop source was necessary.
''')
 put(out/'package-manifest.json',{'schema':'OTERYN_SOURCE_WORLD_CONTROL_PACKAGE/v1','files':{p.relative_to(out).as_posix():sha(p.read_bytes()) for p in sorted(out.rglob('*')) if p.is_file() and p.name!='package-manifest.json'}})
 print(json.dumps({'audited':len(rows),'full_extension_candidates':full,'reference_only':len(rows)-full}))

if __name__=='__main__':
 parser=argparse.ArgumentParser();parser.add_argument('--repo',default='.');args=parser.parse_args();emit(args.repo)
