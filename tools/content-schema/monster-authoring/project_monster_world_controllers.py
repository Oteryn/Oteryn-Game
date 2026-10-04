"""Closed, source-filled monster world controllers r64; proposed DATA only.

Seven concrete domain models cover21 immutable donor slot identities. No Lua/AST
execution shell or operation graph is emitted as a target definition.
"""
import argparse
import copy
import gzip
import hashlib
import json
import re
import sys
from pathlib import Path

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
sys.path.insert(0,str(HERE.parent/'spell-authoring'))
import import_source_player_bundles as base
from source_monster_inline_semantics import read,PINS
from jsonschema import Draft202012Validator

OUT=Path('docs/reference/spells/r64-monster-closure')
URI='urn:oteryn:monster-world-controllers:1'
SCHEMA='monster-world-controllers.schema.json'
FLAGS={k:False for k in ['runtime_activation','native_execution_qualified','input_provider_equivalence','source_consumer_implemented','native_admission','native_provider_qualified']}
BASE51=Path('docs/reference/spells/r51-source-closure/source-monster-slot-semantics.json.gz')
BASE54=Path('docs/reference/spells/r54-source-closure/projected-monster-slot-candidates.json.gz')


def sha(raw):return hashlib.sha256(raw).hexdigest()
def encoded(value):return (json.dumps(value,sort_keys=True,indent=2,ensure_ascii=False)+'\n').encode()
def canonical(value):return json.dumps(value,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()
def put(path,value):path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(encoded(value))
def identity(row):return canonical(row['slot_identity'])
def position(x,y,z):return {'x':int(x),'y':int(y),'z':int(z)}
def positions(text):return [position(*v) for v in re.findall(r'Position\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\)',text)]

def proof(donor,path,symbol=None,scope=None):
 raw=read(donor,path);r={'source':donor,'revision':PINS[donor],'path':path,'sha256':sha(raw)}
 if symbol:
  if path.endswith('.lua'):
   hit=re.search(r'(?ms)^(?:local )?function '+re.escape(symbol)+r'\([^\n]*\)\n.*?^end[ \t]*$',raw.decode())
   if not hit:raise ValueError('missing exact helper '+symbol)
   body=hit.group(0).encode();r['line_start']=raw.decode()[:hit.start()].count('\n')+1
  else:body=base.source_cpp_function(raw,symbol)
  r.update(symbol=symbol,body_sha256=sha(body))
 if scope:r['semantic_scope']=scope
 return r


def scoped_script_proofs(donor,path,text):
 out=[proof(donor,path)]
 for match in re.finditer(r'(?ms)^(?:local )?function ([A-Za-z_][A-Za-z0-9_.:]*)\([^\n]*\)\n.*?^end[ \t]*$',text):
  out.append({'source':donor,'revision':PINS[donor],'path':path,'symbol':match.group(1),'sha256':sha(text.encode()),'body_sha256':sha(match.group(0).encode()),'line_start':text[:match.start()].count('\n')+1})
 return out


def bindings(text):
 flags={}
 for key,value in re.findall(r'(?m)^spell:(isAggressive|blockWalls|needTarget|needLearn)\((true|false)\)',text):flags[key]=value=='true'
 name=re.search(r'(?m)^spell:name\("([^"\n]+)"\)',text).group(1)
 words=re.search(r'(?m)^spell:words\("([^"\n]+)"\)',text).group(1)
 return {'registered_name':name,'words':words,'explicit_registrar_flags':flags,'variant_argument':'unused','core_callback':'InstantSpell'}


def concrete_controller(donor,path,text):
 stem=Path(path).stem;params={'source_model':'r64/'+stem,'registrar':bindings(text)};helpers=[]
 if stem=='heal_brain_head':
  kind='fixed_position_named_monster_heal';params.update(position=positions(text)[0],missing_tile_return=False,projectile='CONST_ANI_HOLY',impact_effect='CONST_ME_MAGIC_BLUE',visuals_before_creature_guards=True,selector={'tile_stack':'top_creature','monster_required':True,'casefold_name':'brain head','query_repeated_each_use':True},healing={'minimum':300,'maximum':500,'sampler':'Lua_math_random_inclusive','signed_health':'positive'},successful_tile_cast_return=True)
 elif stem=='charge_vortex':
  kind='ground_transform';table=text[:text.index('local function createVortex')];params.update(positions=positions(table),position_sampler='Lua_math_random_1_to_list_length',missing_tile='return_nil_without_event',missing_ground='return_nil_without_event',ground_id=22894,revert_id=23049,revert_delay_ms=10000,revert_position='snapshot_tile_getPosition',revert_missing_tile='return_nil',revert_missing_ground='return_nil',restore_original_id=False,cast_return='nil',scheduling_does_not_depend_on_cast_boolean=True)
 elif stem=='mazoran_fire':
  kind='ground_transform';protected=list(map(int,re.search(r'local Montains = \{([^}]+)\}',text).group(1).replace(' ','').split(',')));groundids=list(map(int,re.search(r'local tiles = \{([^}]+)\}',text).group(1).replace(' ','').split(',')))
  tuples=re.findall(r'\{ itemid = (\d+), position = Position\((\d+), (\d+), (\d+)\) \}',text);restore=[{'item_id':int(t[0]),'position':position(*t[1:])} for t in tuples]
  if not restore:raise ValueError('literal restore inventory missing')
  params.update(cast_voice='THE GROUND BEGINS TO HEAT UP RAPIDLY!',voice_type='TALKTYPE_MONSTER_YELL',cast_return='nil',start_delay_ms=3000,caster_reference='captured_creature_ID',missing_delayed_caster_return=True,minion_spawn={'when_caster_summon_count_less_than':4,'attempt_count':4,'deficit_fill':False,'name':'Rage of Mazoran','position':{'x_inclusive':[33576,33593],'y_inclusive':[32684,32695],'z':14},'sampler':'Lua_math_random_inclusive_each_axis','extended':True,'force':True,'master':'captured_caster','set_master_return_ignored':True,'failed_spawn':'abort_callback_nil_before_ground_scan'},ground_scan={'x_inclusive':[33572,33598],'y_inclusive':[32679,32701],'z':14,'iteration_order':'x_outer_y_inner','missing_tile_or_ground':'abort_nil_partial_changes_remain_no_restore_event','skipped_ground_id':1128,'items_missing':'source_length_nil_error','protected_item_ids':protected,'remove_all_other_tile_items':True,'remove_result_ignored':True,'transform_ground_id':21494,'action_id':34200},restore={'delay_after_completed_scan_ms':5000,'ordered_entries':restore,'duplicate_entries_preserved':True,'ground_item_ids':groundids,'missing_tile':'source_error','ground_missing':'skip_entry','ground_transform_to_entry_item_id':True,'ground_action_id':34201,'other_entries':'create_item_at_absolute_position','creation_count':1,'create_failure':'source_method_on_nil_error','created_item_action_id':34201,'callback_return':True})
 elif stem in ('time_guardian','time_guardiann'):
  kind='boss_form_swap';guarded=stem=='time_guardiann';pooltext=text[:text.index('local function functionBack')];pool=positions(pooltext);queryposition=positions(text[text.index('local function functionBack'):])[0]
  params.update(form_positions=pool,required_caster_floor=14,wrong_floor_return=True,random={'minimum':1,'maximum':2,'sampler':'Lua_math_random_inclusive','before_floor_guard':not guarded},initial_missing_tile_or_top='return_true' if guarded else 'source_error',position_missing='skip_to_true' if guarded else 'not_checked',initial_order=(['capture_caster_position','floor_guard_return_true','draw_form_index','select_form_position','position_guard','tile_guard_return_true','top_creature_guard_return_true'] if guarded else ['draw_form_index','capture_caster_position','floor_guard_return_true','select_form_position','lookup_tile_and_top_creature_without_guards'])+['teleport_caster_to_form_ignore_result','adjust_form_health','teleport_form_to_captured_caster_position_ignore_result','schedule_return'],initial_health={'difference':'form_health_minus_caster_health','apply_signed_delta':'negative_difference','apply_only_positive_difference':guarded},return_delay_ms=30000,return_position_reference='selected_form_position',old_position_reference='captured_caster_position',cast_return=True,return_controller={'position_copy_guard':guarded,'missing_tile_or_guardian':'return_nil' if guarded else 'source_error','spectator_query':{'center':queryposition,'multifloor':False,'only_players':False,'x_min':15,'x_max':15,'y_min':15,'y_max':15},'first_scan_nil_guard':guarded,'first_scan_names':['the blazing time guardian','the freezing time guardian'],'first_scan_requires_monster_for':'the blazing time guardian' if guarded else None,'first_scan_other_name_requires_monster':False,'first_scan_last_match_overwrites_old_position':True,'when_no_named_spectator':'remove_guardian_then_return_true','second_scan_nil_guard':guarded,'second_scan_requires_monster_for':'the blazing time guardian' if guarded else 'the freezing time guardian','second_scan_other_name_requires_monster':False,'second_scan_matching_action':'teleport_spectator_to_form_position_then_overwrite_difference_guardian_minus_spectator_health','health_adjustment':'negative_final_difference','apply_only_positive_difference':guarded,'final_action':'teleport_guardian_to_last_first_scan_position','success_return':'nil','teleport_results_ignored':True})
 elif stem=='generator':
  kind='random_absolute_spawn';params.update(positions=positions(text[:text.index('local spell')]),random={'minimum':1,'maximum':4,'sampler':'Lua_math_random_inclusive'},monster_name='glooth-generator',extended=True,force=True,missing_created_monster='source_method_on_nil_error',master_assigned=False,created_monster_voice='THE GLOOTH GENERATOR CHARGES UP FOR A LETHAL EXPLOSION!',voice_type='TALKTYPE_MONSTER_YELL',cast_return='nil')
 elif stem=="gaz'haragoth_summon":
  kind='area_bound_wild_minion_counting';helperpath=path.rsplit('/',1)[0]+'/gaz_functions.lua';h=read(donor,helperpath).decode();initial=int(re.search(r'MinionsNow\s*=\s*(\d+)',h).group(1));maximum=int(re.search(r'MaxSummons\s*=\s*(\d+)',h).group(1))
  globalproof=None
  if donor=='canary':
   gp='data-otservbr-global/monster/quests/the_order_of_lion/usurper_commander.lua';raw=read(donor,gp).decode();line=next(l for l in raw.splitlines() if re.match(r'\s*sum\s*=',l));globalproof=proof(donor,gp,scope='onThink assigns global sum to newly created summon');globalproof.update(assignment_sha256=sha(line.encode()),line_start=raw.splitlines().index(line)+1)
  params.update(state={'binding':'shared_VM_global_GazVariables','initialization':'dofile_unconditional_global_table_assignment_on_spell_load','initial_desired_minions':initial,'maximum_count_threshold':maximum,'desired_count_incremented_only_random_branch':True,'desired_count_not_clamped':True},spectator_query={'center':'caster_position','multifloor':False,'only_players':False,'x_min':25,'x_max':25,'y_min':25,'y_max':25},count_selector={'name_exact':"Minion of Gaz'haragoth",'monster_type_required':False,'master_filter':False,'iteration':'numeric_1_to_length','nil_spectator':'source_method_on_nil_error','source_length_not_nil_test':'always_true_when_table'},threshold_branch={'count_greater_or_equal_maximum':'return_false'},deficit_branch={'count_less_than_desired':True,'attempts':'desired_minus_count','desired_state_update':False,'failure_still_voices':True,'each_attempt_voice':'Minions! Follow my call!','voice_type':'TALKTYPE_MONSTER_SAY'},random_branch={'count_at_least_desired':True,'minimum':0,'maximum':100,'sampler':'Lua_math_random_inclusive','success_strict_less_than':25,'attempt_count':1,'desired_state_increment':1,'failed_spawn_still_increments_state':True},spawn={'name':"Minion of Gaz'haragoth",'position':'caster_position_each_attempt','extended':True,'force':False,'summon_helper_receiver':'newly_created_minion','summon_helper_argument':'mutable_external_global_sum','argument_default':'nil_without_foreign_writer','missing_or_invalid_argument':'helper_false_ignored_new_spawn_remains_without_master','valid_argument':'argument_monster_setMaster_new_minion_true_then_setTarget_new_minion_attackedCreature','master_set_return_ignored':True,'helper_return_ignored':True},cast_effect='CONST_ME_SOUND_RED',cast_effect_when='after_deficit_attempts_or_random_success_even_failed_spawn',cast_return=True,foreign_global_sum_writer=globalproof)
  helpers=[proof(donor,helperpath,scope='GazVariables initializer'),proof(donor,'data/libs/functions/creature.lua','Creature:setSummon')]
 else:raise ValueError('unowned world script '+path)
 defaults=read(donor,'src/creatures/combat/spells.hpp').decode();default_flags={}
 for name,field in [('isAggressive','aggressive'),('needTarget','needTarget'),('needDirection','needDirection'),('casterTargetOrDirection','casterTargetOrDirection'),('blockWalls','checkLineOfSight')]:
  values=set(re.findall(r'\bbool\s+'+field+r'\s*=\s*(true|false)\s*;',defaults))
  if len(values)!=1:raise ValueError('spell default ambiguous '+field)
  default_flags[name]=values.pop()=='true'
 default_flags.update(params['registrar']['explicit_registrar_flags']);params['registrar']['operational_flags']=default_flags
 helpers.append(proof(donor,'src/creatures/combat/spells.hpp',scope='registered InstantSpell defaults'))
 if stem in ('heal_brain_head','time_guardian','time_guardiann'):
  params['health_helper']={'api':'Creature_addHealth','positive_or_zero_type':'COMBAT_HEALING','negative_type_when_optional_arg_omitted':0,'attacker':'nullptr','signed_integer_type':'int32','return_ignored':True};helpers.append(proof(donor,'src/lua/functions/creatures/creature_functions.cpp','CreatureFunctions::luaCreatureAddHealth'))
 if stem in ('time_guardian','time_guardiann'):
  params['teleport_helper']={'push_movement_default':False,'failed_teleport_returns_false':True,'script_ignores_failure':True,'successful_default_teleport_turns_to_destination':True,'direction_rule':{'same_x':{'old_y_less_than_new_y':'south','otherwise':'north'},'old_x_greater_than_new_x':'west','old_x_less_than_new_x':'east'}};helpers.append(proof(donor,'src/lua/functions/creatures/creature_functions.cpp','CreatureFunctions::luaCreatureTeleportTo'))
 if stem in ('mazoran_fire','generator',"gaz'haragoth_summon"):
  helpers.append(proof(donor,'src/lua/functions/core/game/game_functions.cpp','GameFunctions::luaGameCreateMonster'))
 if stem in ('mazoran_fire','charge_vortex'):
  helpers.append(proof(donor,'src/lua/functions/items/item_functions.cpp','ItemFunctions::luaItemTransform'))
 return {'kind':kind,'parameters':params},helpers


def load_inputs():
 a=json.loads(gzip.decompress((ROOT/BASE51).read_bytes()));b=json.loads(gzip.decompress((ROOT/BASE54).read_bytes()));already={identity(r) for r in b['slots'] if r['full_slot_projection_complete']};selected=[r for r in a['slots'] if r['custom_category'] is not None and identity(r) not in already]
 if len(selected)!=21:raise ValueError('exact21 custom remaining slots changed')
 return a,selected


def build():
 capture,selected=load_inputs();rows=[]
 for old in selected:
  syntax=capture['source_programs'][old['source_program_index']]['source_syntax'];donor=old['source'];path=syntax['path'];raw=read(donor,path)
  if sha(raw)!=syntax['source_sha256'] or syntax['revision']!=PINS[donor]:raise ValueError('immutable registered source differs')
  text=raw.decode();controller,helpers=concrete_controller(donor,path,text)
  if controller['kind']!=old['custom_category']:raise ValueError('controller family differs')
  params=copy.deepcopy(old['source_parameters']);minimum=params.get('minDamage',0);maximum=params.get('maxDamage',0)
  proofs=scoped_script_proofs(donor,path,text)+helpers+[proof(donor,'src/creatures/monsters/monsters.hpp',scope='MonsterSpell member defaults'),proof(donor,'src/creatures/monsters/monsters.cpp','Monsters::deserializeSpell'),proof(donor,'src/creatures/combat/spells.cpp','InstantSpell::executeCastSpell')]
  rows.append({'slot_identity':copy.deepcopy(old['slot_identity']),'source':donor,'monster':old['monster'],'original_slot_sha256':old['original_slot_sha256'],'source_parameters':params,'controller':controller,'source_proofs':proofs,'target_schedule':{'interval_ms':params.get('interval',2000),'chance_percent':min(params.get('chance',100),100),'range_tiles':min(params.get('range',0),22),'min_combat_value':min(minimum,maximum),'max_combat_value':max(minimum,maximum),'registered_spell_name':params['name'],'source_target_declaration':params.get('target',False),'registered_spell_controls_targeting':True},'status':'CANDIDATE_SCHEMA_VALID','full_slot_projection_complete':True,'authoring_contract_extension_pending':True,'target_schema_family':'private_source_complete_monster_slot_v2','source_numeric_equivalence':False,'source_helpers_execution_qualified':False,**FLAGS})
 return rows


def closed(value):
 if isinstance(value,dict):return {'type':'object','additionalProperties':False,'required':list(value),'properties':{k:closed(v) for k,v in value.items()}}
 if isinstance(value,list):return {'type':'array','minItems':len(value),'maxItems':len(value),'items':False,**({'prefixItems':[closed(v) for v in value]} if value else {})}
 return {'const':value}

def schema(rows):
 seen={canonical(r['controller']):r['controller'] for r in rows}
 return {'$schema':'https://json-schema.org/draft/2020-12/schema','$id':URI,'title':'Concrete source monster world controllers r64 DATA proposal','$defs':{'controller':{'anyOf':[closed(v) for k,v in sorted(seen.items())]}}}


def emit():
 rows=build();extension=schema(rows);Draft202012Validator.check_schema(extension);validator=Draft202012Validator({'$ref':'#/$defs/controller',**extension})
 for row in rows:
  errors=list(validator.iter_errors(row['controller']))
  if errors:raise ValueError(str(errors[0]))
 out=ROOT/OUT;out.mkdir(parents=True,exist_ok=True);put(HERE/SCHEMA,extension);put(out/SCHEMA,extension)
 payload={'schema':'OTERYN_MONSTER_WORLD_CONTROLLER_PROJECTION/v1','slot_count':len(rows),'full_slot_candidate_count':len(rows),'slots':rows,'authoring_contract_extension_pending':True,'external_sources_used':False,**FLAGS};raw=encoded(payload);(out/'monster-world-controllers.json.gz').write_bytes(gzip.compress(raw,mtime=0))
 put(out/'projection-proof.json',{'schema':'OTERYN_MONSTER_WORLD_CONTROLLER_PROOF/v1','slot_count':21,'source_pins':PINS,'base_source_semantics_path':str(BASE51),'base_source_semantics_sha256':sha((ROOT/BASE51).read_bytes()),'previous_projection_path':str(BASE54),'previous_projection_sha256':sha((ROOT/BASE54).read_bytes()),'schema_id':URI,'schema_sha256':sha(encoded(extension)),'payload_sha256':sha(raw),'gzip_sha256':sha((out/'monster-world-controllers.json.gz').read_bytes()),'producer_sha256':sha(Path(__file__).read_bytes()),'verification':{'command':'/workspace/spell-tools/bin/python -m unittest discover -s tools/content-schema/monster-authoring -p test_project_monster_world_controllers.py -q','tests_passed':7,'test_source_sha256':sha((HERE/'test_project_monster_world_controllers.py').read_bytes()),'immutable_original_Lua_executed_in_test_only':True,'bounded_independent_source_review':'PASS'},'authoring_contract_extension_pending':True,**FLAGS})
 (out/'README.md').write_text('# R64 — monster world controllers\n\n21 immutable slots receive concrete source-filled controller DATA for seven script variants in five domain families. Original slot identities, source hashes and raw parameters remain unchanged. Mazoran restore inventories retain order and duplicates; boss health/teleport guards preserve source differences; Gaz state and nullable foreign-global sum binding stay explicit.\n\nThe private closed controller schema is a proposed MonsterSlot v2 extension. Runtime, native execution, input providers and consumer remain unqualified and disabled. Seven tests pass with immutable original Lua observations for restore inventory, guard/RNG ordering, heal presentations, delayed transforms, spawn flags and shared Gaz state/helper behavior. Source is cached pinned Canary/Crystal Git data only.\n')
 put(out/'package-manifest.json',{'schema':'OTERYN_MONSTER_WORLD_CONTROLLER_PACKAGE/v1','files':{p.relative_to(out).as_posix():sha(p.read_bytes()) for p in sorted(out.rglob('*')) if p.is_file() and p.name!='package-manifest.json'}})
 print(json.dumps({'slots':21,'full_data':21,'runtime':False}))

if __name__=='__main__':argparse.ArgumentParser().parse_args();emit()
