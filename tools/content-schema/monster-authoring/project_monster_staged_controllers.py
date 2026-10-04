"""R66 closed domain controllers for 83 unresolved registered monster slots.

Full typed stages/world effects, with donor failures preserved; no executable Lua
or generic operation language and no runtime admission.
"""
import copy
import sys
import gzip
import hashlib
import json
import re
from functools import lru_cache
from pathlib import Path
from jsonschema import Draft202012Validator
import project_monster_slot_candidates as previous
from source_monster_inline_semantics import read, PINS, LINKS, SOURCE_ROOT
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / 'spell-authoring'))
import import_source_player_bundles as player_defaults

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
OUTPUT = ROOT / 'docs/reference/spells/r66-monster-closure'
SCHEMA = HERE / 'monster-staged-controllers.schema.json'
EXCLUDED = set('lisa_skill_reducer lisa_heal spider_queen_wrap walker_skill_reducer minotaur_cult_prophet_mass_healing rotthing_wave plagirath_bog icicle_heal glooth_fairy_healing omrafir_healing_2 the_welter_heal tyrn_heal ignite candy_horror_wave rootkraken_deathholy rootkraken_rootearth foam_splash ratmiral_fire_wave ratmiral_ball smelly_cheese_berserk angry_orc_ancestor_spirit_rooted angry_orc_ancestor_spirit_fear'.split())


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()


def position(x, y, z):
    return {'x': x, 'y': y, 'z': z}


@lru_cache(maxsize=1)
def cohort():
    source, registrations = previous.load()
    r54 = json.loads(gzip.decompress((ROOT / 'docs/reference/spells/r54-source-closure/projected-monster-slot-candidates.json.gz').read_bytes()))
    full = {canonical(row['slot_identity']) for row in r54['slots'] if row['full_slot_projection_complete']}
    links = {canonical(row['slot_identity']): row for row in (json.loads(line) for line in gzip.decompress(LINKS.read_bytes()).splitlines())}
    result = []
    for slot in source['slots']:
        if slot['custom_category'] is not None or slot.get('source_program_index') is None or canonical(slot['slot_identity']) in full:
            continue
        syntax = source['source_programs'][slot['source_program_index']]['source_syntax']
        stem = Path(syntax['path']).stem
        if stem in EXCLUDED:
            continue
        registration = next(value for key, value in registrations.items() if isinstance(key, tuple) and key[:2] == (slot['source'], syntax['path']))
        result.append((slot, syntax, registration, links[canonical(slot['slot_identity'])]))
    if len(result) != 83:
        raise ValueError('R66 closed membership changed')
    return result


def domain_parameters(stem, donor, text):
    """Actual finite domain models: properties describe stages, not opcodes."""
    if stem in ('summon_challenge', 'summonchallenge'):
        return {'cast_return': 'combat_execute', 'target_callback': {'taunt_target_to_caster': True,
            'cooldown_ms': 8000 if stem == 'summonchallenge' else 6000,
            'missing_caster_or_target': 'callback_false', 'underlying_challenge_method_return_ignored_by_global_wrapper': True,
            'callback_return': 'doChallengeCreature_boolean_result',
            'helper_contract': {'creature_default_result':False,'monster_summon_target_rejected':True,
                'select_target_requires': ['not_removed','attackable','not_protection_zone','visible','same_floor','in_target_list'],
                'crystal_can_target_required':donor=='crystal', 'crystal_disconnected_player_rejected':donor=='crystal',
                'canary_expired_target_reference_removed_and_rejected':donor=='canary',
                'canary_perf_test_friendly_fire_override':donor=='canary',
                'nondefault_faction_requires_enemy_faction':True, 'login_protection_check':'isLoginProtected' if donor=='canary' else 'checkLoginDelay',
                'hostile_monster_sets_attacked_creature_then_checks_attack':True,
                'attack_check':'synchronous' if donor=='canary' else 'dispatcher_event',
                'select_target_result':'setFollowCreature_result', 'on_success_focus_duration':'cooldown_ms',
                'on_success_target_change_ticks':0, 'live_player_battle_healing_call':donor=='canary',
                'battle_healing_owner':'PlayerWheel.healIfBattleHealingActive' if donor=='canary' else 'none'}}}
    if stem == 'doctor_marrow_explosion':
        return {'target_selection': 'creature_variant_number_lookup', 'missing_target_return': False,
            'target_position': 'snapshot_at_cast', 'initial_target_voice': "You are being targeted by Doctor Marrow's explosion!",
            'initial_caster_effect': 'CONST_ME_ORANGE_ENERGY_SPARK', 'initial_target_effect': 'CONST_ME_ORANGETELEPORT',
            'paralyze_pulses': {'first_ms': 0, 'last_ms': 4000, 'interval_ms': 100, 'count': 41,
                'center': 'snapshot_target_position', 'range_xy': 6, 'same_floor': True, 'players_only': True,
                'effective_first_ms':100, 'effective_first_two_at_ms':100, 'scheduler_minimum_ms':100,
                'duration_ms': 500, 'speed_formula_coefficients': ['-0.94', '0', '-0.97', '0'], 'caster_required': False},
            'warning_repeat': {'delay_ms': 2000, 'caster': 'id_lookup_if_present', 'caster_position': 'fresh',
                'target_position': 'snapshot', 'caster_effect': 'CONST_ME_ORANGE_ENERGY_SPARK', 'target_effect': 'CONST_ME_ORANGETELEPORT'},
            'blast': {'delay_ms': 4000, 'caster': 'id_lookup_if_present', 'alive_check': False,
                'damage_draw': [-7000, -3500], 'draw_source': 'negated_uniform_3500_7000',
                'critical_chance_percent': 10, 'critical_multiplier': '1.5', 'state_storage': 'script_shared_damage_and_crit',
                'target_filter': 'players_and_any_owned_creature', 'wild_nonplayer_nil_master': 'lua_nil_master_isPlayer_error',
                'distance': 'floor_source_Position_getDistance_from_fresh_caster_to_target',
                'distance_scale': '1_over_2_pow_distance', 'attacker': 'zero', 'damage_type': 'earth',
                'critical_effect': 'CONST_ME_CRITICAL_DAMAGE', 'center': 'snapshot_target_position'}, 'cast_return': True}
    if stem == 'sapling_explode':
        return {'removal': {'delay_ms': 1, 'effective_delay_ms':100, 'identity': 'creature_uid', 'missing_creature': 'return_false'},
                'removal_scheduled_before_combat': True, 'cast_return': 'combat_execute'}
    if stem == 'priestess_firering':
        return {'cast_return': 'combat_execute', 'target_callback': None,
            'misspelled_shoot_effect': 'nil_parameter_zero_sets_combat_type_to_CONST_ANI_FIRE_numeric'}
    if stem == 'rotthing_shaper':
        return {'combat_iteration': 'source_pairs_unspecified_order', 'combat_ids': ['0', '1'],
            'combat_false_ignored': True, 'cast_return': True,
            'literal_damage_sign': 'zero_to_positive_health_bounds', 'unused_setFormula_coefficients_preserved': True}
    if stem == 'ragiaz_transform':
        return {'required_floor': 13, 'wrong_floor_return': 'nil', 'effect': 172,
            'voice': 'Ragiaz encase himself in bones to regenerate.', 'caster_destination': position(33487,32333,14),
            'caster_health_delta': 1000, 'capsule_tile': position(33485,32333,14),
            'capsule_selector': 'top_creature', 'capsule_missing': 'unguarded_lua_error',
            'capsule_destination': 'snapshot_original_caster_position',
            'order': ['original_effect','voice','caster_teleport','caster_heal','capsule_teleport'], 'cast_return': 'nil'}
    if stem == 'zamulosh_invisible':
        return {'room_from': position(33634,32749,11), 'room_to': position(33654,32765,11),
            'iteration': 'inclusive_xyz', 'selector': 'top_creature_each_tile', 'missing_tile': 'unguarded_lua_error',
            'target_filter': 'monster_name_lowercase_zamulosh', 'effect': 'CONST_ME_TELEPORT',
            'condition': 'invisible', 'duration_ms': 10000, 'cast_return': 'nil'}
    if stem in ('zamulosh_tp','maxxen_teleport'):
        return {'before_effect': 'CONST_ME_POFF', 'after_effect': 'CONST_ME_TELEPORT',
            'destination': position(33644,32757,11) if stem == 'zamulosh_tp' else {'x_uniform': [33704,33718], 'y_uniform': [32040,32053], 'z': 15},
            'teleport_failure_ignored': True, 'cast_return': 'nil'}
    if stem in ('eruption_of_destruction_explosion', "gaz'haragoth_death", 'tenebris_ultimate'):
        eruption = stem.startswith('eruption'); tenebris = stem == 'tenebris_ultimate'
        params = {'delay_ms': 7000 if eruption else 4000 if tenebris else 5000,
            'event_caster': 'id_lookup_if_present', 'event_health_positive_required': eruption or tenebris,
            'event_center': 'fresh_caster_position', 'event_variant_input': 'ignored',
            'target_callback': {'selector': 'tile_creatures_array' if tenebris else 'legacy_stack_creatures_by_numeric_uid',
                'self_exclusion': 'compare_creature_ids' if tenebris else 'numeric_uid_vs_userdata_ineffective_comparison',
                'players_allowed_base_vocations': ['sorcerer','druid','paladin','knight'], 'monsters_allowed': True,
                'damage_type': 'fire' if eruption else 'death' if tenebris else 'energy',
                'signed_bounds': [-4000,-6000] if eruption else [-2200,-2500] if tenebris else [-30000,-30000],
                'tile_effect': 'CONST_ME_FIREAREA' if eruption else 'CONST_ME_MORTAREA' if tenebris else 'CONST_ME_PURPLEENERGY',
                'caster_remove_each_tile': eruption, 'missing_tile': 'callback_true' if tenebris else 'unguarded_lua_error'},
            'cast_return': True}
        if eruption:
            params['event_before_combat'] = {'master': 'unchecked_caster_master', 'master_health_add_uniform': [20000,30000],
                'master_health_add_flags': [True,True], 'spawn': 'demon', 'spawn_position': 'fresh_caster',
                'spawn_flags': [True,True], 'order': ['master_heal','spawn','combat']}
        elif tenebris:
            params['gather'] = {'center': position(32912,31599,14), 'range_xy': 12, 'same_floor': True,
                'players': 'teleport_if_not_already_center', 'boss_name_lowercase': 'lady tenebris',
                'boss': 'teleport_if_not_center_then_move_lock_true', 'other_creatures': 'skip'}
            params['initial_voice'] = 'LADY TENEBRIS BEGINS TO CHANNEL A POWERFULL SPELL! TAKE COVER!'
            params['event_before_combat'] = {'move_lock': False}
        else:
            params['initial_voice'] = "Gaz'haragoth begins to channel DEATH AND DOOM into the area! RUN!"
            params['event_voice'] = "Gaz'haragoth calls down: DEATH AND DOOM!"
        return params
    if stem == 'time_guardian_lost_time':
        return {'required_floor': 15, 'wrong_floor_return': True,
            'name_to_spawn': {'the freezing time guardian': 'lost time', 'the blazing time guardian': 'time waster'},
            'name_case': 'lowercase', 'offset_x_uniform': [-2,2], 'offset_y_uniform': [-2,2],
            'floor': 'caster_current', 'spawn_failure_ignored': True, 'cast_return': True}
    if stem == 'outburst_explode':
        return {'gather': {'center': position(32234,31285,14), 'range_xy': 10, 'players_only_query': True,
                'player_destination': position(32234,31280,14), 'monster_name_exact': 'Charging Outburst',
                'monster_destination': position(32234,31279,14), 'monster_branch_query_unreachable': True},
            'combat': 'immediate_current_caster_position', 'storage_set': 'HeartOfDestruction.OutburstChargingKilled', 'storage_value': 1,
            'removal_delay_ms': 1000, 'removal_identity': 'uid_lookup_if_present',
            'spawn_name': 'Outburst', 'spawn_position': position(32234,31284,14), 'spawn_flags': [False,True],
            'spawn_health': 'positive_OutburstHealth_storage_else_zero_then_add_negative_current_health',
            'spawn_health_second_argument': 'COMBAT_PHYSICALDAMAGE_as_bool',
            'order': ['gather','combat','storage','schedule_remove','spawn','adjust_spawn_health'], 'cast_return': True}
    if stem == 'energy_pulse_explosion':
        return {'return_values': ['combat_result','remove_result'], 'core_consumes_return_value_index': 0,
                'order': ['combat','remove_caster'],
                'remove_even_if_combat_false': True}
    if stem == 'glooth-generator_summon':
        return {'delay_ms': 14000, 'caster': 'id_lookup_if_present', 'spawn_name': 'Energy Pulse',
            'spawn_position': 'fresh_caster_position', 'spawn_flags': [True,True],
            'voice': 'The fully charged generator explodes in a blast!', 'order': ['spawn','voice','remove_caster'],
            'spawn_failure_ignored': True, 'cast_return': True}
    if stem == 'omrafir_beam':
        return {'guard_condition': {'type': 'regeneration', 'id': 'default', 'sub_id': 88888},
            'guard_present_return': 'nil', 'guard_install': {'duration_ms': 5000, 'health_gain_source': '0.01',
                'health_gain_integer': 0, 'health_ticks_ms': 5000}, 'initial_voice': 'OMRAFIR INHALES DEEPLY!',
            'delay_ms': 4000, 'caster': 'id_lookup_if_present', 'direction': 'fresh_at_event',
            'direction_to_combat': {'0': '0','1': '1','2': '2','3': '3'}, 'center': 'fresh_caster_position',
            'unknown_direction': 'no_combat_then_voice', 'event_voice': 'OMRAFIR BREATHES INFERNAL FIRE', 'cast_return': True}
    if stem == 'spell-megalomania_blue':
        return {'cast_center': 'fresh_caster_position', 'per_area_tile_callback': True,
            'zone_name': "boss.goshnar's-megalomania-purple", 'zone_positions': 'load_time_zone_getPositions_snapshot' if 'local zonePositions' in text else 'external_global_zonePositions_default_nil',
            'zone_missing_at_load': 'unguarded_lua_error' if 'local zonePositions' in text else 'not_looked_up',
            'external_global_writers_qualified':False, 'external_global_on_nil':'ipairs_lua_error',
            'for_each_callback_full_zone_iteration': True, 'zone_tile_selector': 'top_creature_player',
            'require_tile_and_ground': True, 'excluded_ground_id': 409,
            'signed_direct_health_delta': -6000, 'health_delta_argument_2': 'COMBAT_DEATHDAMAGE_as_bool',
            'callback_tile_effect': 'CONST_ME_BLACKSMOKE', 'cast_return': 'combat_execute'}
    if stem in ('spell-fire_beam_megalomania','spell-fire_beam_cruelty'):
        return {'zone_name': "boss.goshnar's-cruelty" if stem.endswith('cruelty') else "boss.goshnar's-megalomania-purple",
            'zone_missing_return': False, 'zone_player_selection': 'uniform_random_one', 'no_players_return': True,
            'outfit': {'lookType':242,'lookHead':0,'lookBody':0,'lookLegs':0,'lookFeet':0,'lookAddons':0},
            'outfit_duration_ms':7000, 'outfit_effect':'CONST_ME_MAGIC_BLUE', 'delay_ms':7000,
            'condition_instance':'shared_soul_war_outfit', 'event_caster':'id_lookup_if_present',
            'event_target':'player_id_lookup_if_present', 'event_position':'fresh_target_position',
            'event_order':['combat','target_remove_outfit_condition'], 'missing_caster_leaves_outfit_until_expiry':True,
            'tile_callback': {'selector':'top_creature_player', 'missing_tile':'callback_true',
                'health_uniform_positive':[2300,3000], 'health_default_broadcast':True}, 'cast_return':True}
    if stem == 'spell-eye_beam':
        return {'cast_return':'combat_execute', 'tile_callback': {'selector':'top_creature',
            'missing_tile':'unguarded_getTopCreature_error_before_tile_check', 'target_filter':'monster_name_exact_Poor Soul',
            'health_delta':-1000, 'health_default_broadcast':True}}
    if stem == 'the_welter_summon2':
        return {'spectators':{'center':'fresh_caster_position','range_xy':50,'same_floor':True,'players_only':False},
            'count_names_exact':['Egg','Spawn Of The Welter'], 'count_includes_owned_creatures':True,
            'spawn_below_count':10, 'spawn_name':'Egg', 'spawn_flags':[False,True], 'spawn_position':'fresh_caster_position',
            'spawn_effect':'CONST_ME_GREEN_RINGS', 'spawn_failure_ignored':True, 'cast_return':True}
    if stem == 'sugar_daddy_cake':
        return {'target_guard':'caster_current_target_exists_and_player', 'guard_failure_return':False,
            'cast_variant':'original_unchanged_not_guard_target_position', 'cast_return':'combat_execute',
            'target_conditions':[{'group_sub_id':3,'duration_ms':5000},{'group_sub_id':2,'duration_ms':5000}]}
    if stem == 'herald_of_fire_firefields':
        return {'grid':{'from':position(32492,32652,15),'to':position(32500,32660,15),'step_xy':2,'point_count':25},
            'telegraph_effect':53557, 'delay_ms':1500, 'event_caster_required':False,
            'missing_tile':'event_return_nil', 'old_field_ids_remove_first_match_each':[2118,2119,2120],
            'create_field_id':2118,'create_count':1,'created_field_decay':True,
            'event_order':['remove_old_fields','create_field','decay_if_created'], 'cast_return':True}
    raise ValueError('Unrepresented R66 script family: ' + stem)


@lru_cache(maxsize=2)
def donor_converter(donor):
    return previous.converter(donor)


def combat_models(registration, donor, slot):
    converter = donor_converter(donor)
    result = []
    for index, combat in registration['conversion'].get('reference_combats', {}).items():
        notes = []; engine = converter.engine_params(combat.get('param_calls', []), notes)
        formula = combat.get('formula')
        result.append({'source_combat_id':index, 'engine_parameters':engine, 'source_engine_conversion_notes':notes,
            'area':combat.get('area'), 'damage': {'kind':'signed_literal_setFormula' if formula else 'registered_monster_slot_bounds',
                'minimum':formula[1] if formula else slot['source_parameters'].get('minDamage',0),
                'maximum':formula[3] if formula else slot['source_parameters'].get('maxDamage',0),
                'ignored_b_coefficients': [formula[2],formula[4]] if formula else [], 'cast_roll':'engine_world_random'},
            'callbacks':sorted(combat.get('callbacks', {})),
            'callback_semantics':'domain_controller_parameters',
            'attached_conditions': [{'type': condition['type'],
                'sub_id': next(args[1] for method,args in condition['calls'] if method=='setParameter' and args[0]=='CONDITION_PARAM_SUBID'),
                'duration_ms': next(args[1] for method,args in condition['calls'] if method=='setParameter' and args[0]=='CONDITION_PARAM_TICKS')}
                for condition in combat.get('conditions', [])]})
    return result


def build():
    rows = []
    defaults = {donor: player_defaults.default_fields(SOURCE_ROOT / donor, pin)[0] for donor,pin in PINS.items()}
    for slot, syntax, registration, link in cohort():
        donor = slot['source']; path = syntax['path']; text = read(donor,path)
        if sha(text) != syntax['source_sha256'] or sha(text) != registration['provenance']['sha256']:
            raise ValueError('Exact staged spell source identity changed')
        monster = link['monster_source']; monster_bytes = read(donor,monster['path'])
        if sha(monster_bytes) != monster['sha256']:
            raise ValueError('Monster source identity changed')
        stem = Path(path).stem; params = domain_parameters(stem, donor, text.decode())
        params['source_model'] = 'r66_' + stem.replace('-','_').replace("'",'')
        params['event_scheduler_minimum_delay_ms'] = 100
        params['combat_models'] = combat_models(registration, donor, slot)
        proofs = [{'source':donor,'revision':PINS[donor],'path':path,'sha256':sha(text),'scope':'exact_registered_controller'},
                  {'source':donor, **monster, 'scope':'exact_slot_owner'}]
        for helper in ('src/lua/functions/core/game/global_functions.cpp','src/lua/functions/creatures/creature_functions.cpp',
                       'src/creatures/combat/combat.cpp','src/creatures/monsters/monsters.cpp','src/creatures/combat/spells.hpp','src/creatures/monsters/monster.cpp', 'src/creatures/creature.hpp'):
            proofs.append({'source':donor,'revision':PINS[donor],'path':helper,'sha256':sha(read(donor,helper)),
                'scope':'provider_conversion_and_callbacks','runtime_qualified':False})
        if stem == 'outburst_explode':
            storage_path = 'data-otservbr-global/lib/core/storages.lua' if donor=='canary' else 'data-global/lib/core/storages.lua'
            storage_bytes = read(donor,storage_path)
            global_text = storage_bytes.decode().split('GlobalStorage = {',1)[1]
            block = re.search(r'HeartOfDestruction\s*=\s*\{(.*?)\n\s*\}',global_text,re.S).group(1)
            keys = {name:int(re.search(r'\b'+name+r'\s*=\s*([0-9]+)',block).group(1))
                    for name in ('OutburstHealth','OutburstChargingKilled')}
            params['global_storage'] = {'charging_killed':{'source_symbol':'GlobalStorage.HeartOfDestruction.OutburstChargingKilled','key':keys['OutburstChargingKilled']},
                'health':{'source_symbol':'GlobalStorage.HeartOfDestruction.OutburstHealth','key':keys['OutburstHealth']},
                'health_read_policy':'read_positive_guard_then_fresh_second_read_else_zero',
                'positive_guard_and_value_read_atomic':False}
            proofs.append({'source':donor,'revision':PINS[donor],'path':storage_path,'sha256':sha(storage_bytes),
                'scope':'GlobalStorage.HeartOfDestruction.OutburstHealth_and_OutburstChargingKilled'})
        if stem.startswith('spell-fire_beam'):
            helper = 'data-otservbr-global/lib/quests/soul_war.lua' if donor=='canary' else 'data-global/lib/quests/soul_war.lua'
            proofs.append({'source':donor,'revision':PINS[donor],'path':helper,'sha256':sha(read(donor,helper)),
                'scope':'applyZoneEffect_random_player_outfit_and_delayed_callback'})
        flag_defaults = {'needTarget':'needTarget','needDirection':'needDirection', 'needCasterTargetOrDirection':'casterTargetOrDirection',
            'isSelfTarget':'selfTarget','isAggressive':'aggressive','blockWalls':'checkLineOfSight','needLearn':'learnable','cooldown':'cooldown'}
        calls = registration['conversion'].get('spell_calls',{})
        effective_flags = {field: calls.get(field,[defaults[donor][default]])[-1] for field,default in flag_defaults.items()}
        effective_flags['cooldown'] = int(effective_flags['cooldown'])
        flags = {'runtime_activation':False,'native_execution_qualified':False,'native_provider_qualified':False,
            'source_consumer_implemented':False,'authoring_contract_extension_pending':True,
            'target_schema_family':'private_monster_slot_v2', 'native_admission':False,
            'input_provider_equivalence':False,'canonical_selection_changed':False,'native_identity_allocation':False}
        controller = {'kind':'r66_staged_monster', 'parameters':params}
        rows.append({'slot_identity':copy.deepcopy(slot['slot_identity']),'source':donor,'monster':slot['monster'],
            'original_slot_sha256':slot['original_slot_sha256'],'source_parameters':copy.deepcopy(slot['source_parameters']),
            'controller':controller,'source_proofs':proofs,'status':'CANDIDATE_SCHEMA_VALID',
            'full_slot_projection_complete':True,'required_operations_unrepresented':[],
            'source_runtime_error_known': params.get('zone_positions') == 'external_global_zonePositions_default_nil',
            'source_runtime_error': 'Conditional: ipairs(zonePositions=nil) in target tile callback; foreign global writers remain unqualified' if params.get('zone_positions') == 'external_global_zonePositions_default_nil' else None,
            'target_schedule':{'controller':controller, 'range_tiles':min(slot['source_parameters'].get('range',0),22),
                'need_target':slot['source_parameters'].get('target',False),
                'registration_flags':copy.deepcopy(registration['conversion'].get('spell_calls',{})),
                'effective_registration_flags':effective_flags,
                'interval_ms':slot['source_parameters'].get('interval',2000),
                'chance_percent':min(slot['source_parameters'].get('chance',100),100)}, **flags})
    return rows


def closed(value):
    if isinstance(value,dict):
        return {'type':'object','additionalProperties':False,'required':sorted(value), 'properties':{key:closed(item) for key,item in value.items()}}
    if isinstance(value,list):
        return {'type':'array','prefixItems':[closed(item) for item in value], 'items':False,'minItems':len(value),'maxItems':len(value)} if value else {'type':'array','items':False,'maxItems':0}
    return {'type':'null' if value is None else 'boolean' if isinstance(value,bool) else 'integer' if isinstance(value,int) else 'number' if isinstance(value,float) else 'string','const':value}


def write():
    rows = build(); unique = {canonical(row['controller']):row['controller'] for row in rows}
    schema = {'$schema':'https://json-schema.org/draft/2020-12/schema','$id':'urn:oteryn:monster-staged-controllers:1',
        '$defs':{'controller':{'oneOf':[closed(item) for item in unique.values()]}}, '$ref':'#/$defs/controller'}
    Draft202012Validator.check_schema(schema)
    validator=Draft202012Validator(schema)
    for row in rows:validator.validate(row['controller'])
    OUTPUT.mkdir(parents=True,exist_ok=True)
    raw=(json.dumps(schema,sort_keys=True,indent=2)+'\n').encode();SCHEMA.write_bytes(raw)
    (OUTPUT/SCHEMA.name).write_bytes(raw)
    packet={'schema':'OTERYN_MONSTER_STAGED_CONTROLLERS/v1','slot_count':len(rows),'full_slot_candidate_count':len(rows),'records':rows,
        'runtime_activation':False,'native_execution_qualified':False,'source_consumer_implemented':False,
        'authoring_contract_extension_pending':True,'source_model_count':len(unique)}
    data=canonical(packet);encoded=gzip.compress(data,mtime=0);(OUTPUT/'monster-staged-controllers.json.gz').write_bytes(encoded)
    receipt={'slot_count':len(rows),'full_slot_candidate_count':len(rows),'controller_schema_sha256':sha(raw),
        'gzip_sha256':sha(encoded),'payload_sha256':sha(data),'producer_sha256':sha(Path(__file__).read_bytes()),
        'source_revisions':PINS,'runtime_activation':False,'native_execution_qualified':False,
        'source_consumer_implemented':False,'authoring_contract_extension_pending':True}
    (OUTPUT/'projection-proof.json').write_text(json.dumps(receipt,sort_keys=True,indent=2)+'\n')
    manifest={'files':{path.name:sha(path.read_bytes()) for path in sorted(OUTPUT.iterdir()) if path.is_file() and path.name!='package-manifest.json'}}
    (OUTPUT/'package-manifest.json').write_text(json.dumps(manifest,sort_keys=True,indent=2)+'\n')
    return packet


if __name__=='__main__':write()
