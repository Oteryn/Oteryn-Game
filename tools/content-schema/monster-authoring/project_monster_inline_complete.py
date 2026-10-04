"""Complete typed inline MonsterSlot DATA for ten undefined signed-health slots.

The donor's undefined type and mixed signs are preserved. No wiki repair, valid
life/mana alias, executable source text or generic program model is introduced.
"""
import argparse
import copy
import gzip
import hashlib
import json
from functools import lru_cache
from pathlib import Path
import re

import jsonschema
import source_monster_inline_semantics as inline
import spell_scripts
import build_monster_presentation_closure as presentation

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
R51=ROOT/'docs/reference/spells/r51-source-closure/source-monster-slot-semantics.json.gz'
R54=ROOT/'docs/reference/spells/r54-source-closure/projected-monster-slot-candidates.json.gz'
OUT=ROOT/'docs/reference/spells/r63-monster-closure'
SCHEMA=HERE/'monster-inline-controller.schema.json'
FLAGS={'runtime_activation':False,'native_execution_qualified':False,'native_admission':False,'input_provider_equivalence':False,'source_consumer_implemented':False,'canonical_selection_changed':False,'native_identity_allocation':False,'authoring_contract_extension_pending':True}


def sha(raw):return hashlib.sha256(raw).hexdigest()
def canonical(value):return json.dumps(value,ensure_ascii=False,sort_keys=True,separators=(',',':')).encode()
def object_schema(properties):return {'type':'object','additionalProperties':False,'properties':properties,'required':list(properties)}
def fixed(value):
    if isinstance(value,dict):return object_schema({key:fixed(child) for key,child in value.items()})
    if isinstance(value,list):
        if not value:return {'type':'array','items':False,'minItems':0,'maxItems':0}
        return {'type':'array','prefixItems':[fixed(child) for child in value],'items':False,'minItems':len(value),'maxItems':len(value)}
    return {'const':value}


def cohort():
    first=json.loads(gzip.decompress(R51.read_bytes()));prior=json.loads(gzip.decompress(R54.read_bytes()))
    full={canonical(row['slot_identity']) for row in prior['slots'] if row['full_slot_projection_complete'] and row['canonical_normalization'] and row['canonical_normalization']['decision']=='D25'}
    source=[row for row in first['slots'] if row['source_program_index'] is None]
    if len(source)!=19 or len(full)!=9:raise ValueError('immutable inline19/D25nine partition differs')
    remaining=[row for row in source if canonical(row['slot_identity']) not in full]
    if len(remaining)!=10 or len({canonical(row['slot_identity']) for row in remaining})!=10:raise ValueError('requires exact remaining inline10')
    return remaining


def function_scope(text,signature):
    start=text.find(signature)
    if start<0:raise ValueError('missing immutable helper '+signature)
    start=text.rfind('\n',0,start)+1
    end=text.find('\n}',start)
    if end<0:raise ValueError('unclosed immutable helper '+signature)
    scope=text[start:end+2]
    return scope,text[:start].count('\n')+1,text[:end].count('\n')+2


@lru_cache(maxsize=2)
def helpers(donor):
    required={
        'src/creatures/monsters/monsters.cpp': {'Monsters::deserializeSpell(':['std::min(spell->minCombatValue, spell->maxCombatValue)','std::max(spell->minCombatValue, spell->maxCombatValue)','combatPtr->setParam(COMBAT_PARAM_TYPE, spell->combatType)','COMBAT_FORMULA_DAMAGE']},
        'src/creatures/combat/combat.cpp':{
            'Combat::getCombatDamage(':['normal_random(','formulaType == COMBAT_FORMULA_DAMAGE'],
            'AreaCombat::setupArea(int32_t radius)':['cell > 0 && cell <= radius','setupArea(list, 13)'],
            'Combat::CombatHealthFunc(':['CombatDamage damage = *data','combatChangeHealth'],
            'Combat::CombatFunc(':['tmpDamage = *data','func(caster, creature, params, &tmpDamage)'],
        },
        'src/creatures/combat/spells.cpp':{'CombatSpell::castSpell(const std::shared_ptr<Creature> &creature, const std::shared_ptr<Creature> &target)':['combat->hasArea()','needTarget && target','combat->doCombat(creature, target)']},
        'src/game/game.cpp':{'Game::combatChangeHealth(':['damage.primary.value > 0','target->gainHealth(attacker, damage.primary.value)','damage.primary.type != COMBAT_UNDEFINEDDAMAGE']},
        'src/creatures/monsters/monster.cpp':{'Monster::canUseSpell(':['sb.speed > attackTicks','attackTicks % sb.speed >= interval','sb.range != 0','Position::getDistanceX(pos, targetPos)','Position::getDistanceY(pos, targetPos)']},
        'src/utils/tools.cpp':{'normal_random(int32_t minNumber, int32_t maxNumber)':['std::normal_distribution<float> normalRand(0.5f, 0.25f)','v < 0.0 || v > 1.0','std::lround(v * (b - a))']},
    }
    proofs=copy.deepcopy(inline.source_proofs(donor));scopes=[]
    for path,functions in required.items():
        raw=inline.read(donor,path);text=raw.decode()
        if path not in {row['path'] for row in proofs}:proofs.append({'source':donor,'revision':inline.PINS[donor],'path':path,'scope':'signed-health controller donor constructor/execution contract','sha256':sha(raw)})
        for signature,literals in functions.items():
            scope,start,end=function_scope(text,signature)
            for literal in literals:
                if literal not in scope:raise ValueError('immutable signed-health helper differs: '+signature+' '+literal)
            scopes.append({'path':path,'function':signature,'revision':inline.PINS[donor],'start_line':start,'end_line':end,'scope_sha256':sha(scope.encode()),'qualified_literals':literals})
    # Bind CombatParams defaults as immutable bytes rather than mutable Git checkout files.
    raw=inline.read(donor,'src/creatures/combat/combat.hpp');text=raw.decode()
    for literal in ['origin = ORIGIN_SPELL','blockedByArmor = false','blockedByShield = false','aggressive = true','useCharges = false']:
        if literal not in text:raise ValueError('immutable CombatParams default differs: '+literal)
    proofs.append({'source':donor,'revision':inline.PINS[donor],'path':'src/creatures/combat/combat.hpp','scope':'CombatParams constructor defaults','sha256':sha(raw)})
    return proofs,scopes


@lru_cache(maxsize=2)
def radius_matrix(donor):
    text=inline.read(donor,'src/creatures/combat/combat.cpp').decode();scope,_,_=function_scope(text,'AreaCombat::setupArea(int32_t radius)')
    block=re.search(r'area\[13\]\[13\]\s*=\s*\{(.*?)\};',scope,re.S)
    rows=[[int(cell) for cell in row.split(',') if cell.strip()] for row in re.findall(r'\{([^{}]+)\}',block.group(1))]
    if len(rows)!=13 or any(len(row)!=13 for row in rows):raise ValueError('source radius stencil differs')
    return rows


def geometry(donor,normalized):
    steps=normalized['area_construction_order']
    if any(step['shape']!='radius' for step in steps):raise ValueError('inline10 no longer radius-or-target only')
    radius=steps[-1]['radius'] if steps else 0
    cells=[]
    if radius:
        for y,row in enumerate(radius_matrix(donor)):
            for x,cell in enumerate(row):
                if cell>0 and cell<=radius:cells.append({'dx':x-6,'dy':y-6,'origin':cell==1})
    return {'kind':'target_position_area' if radius else 'direct_creature_target','radius_constructor_value':radius,'active_cells':cells,
            'area_construction_order':copy.deepcopy(steps),'target_required_flag':normalized['need_target'],'direction_required_flag':normalized['need_direction'],
            'area_anchor_with_target':'target_position','area_anchor_without_target':'caster_position_or_direction',
            'single_without_target':'dispatch_null_target_to_donor_direct_combat','same_floor':True,'tile_and_target_legality':'donor_combat_legality_provider',
            'target_caster_or_topmost':False,'base_draw_scope':'one_per_combat_dispatch','per_target_damage_copy':True}


@lru_cache(maxsize=2)
def sound_helper(donor):
    helper=presentation.HelperCapture(inline.read(donor,presentation.HELPER).decode())
    utilities=inline.read(donor,presentation.UTILS).decode();definitions=inline.read(donor,presentation.HEADER).decode()
    constants={**spell_scripts.enum_values(utilities,'MagicEffectClasses'),**spell_scripts.enum_values(utilities,'ShootType_t'),**presentation.enum(definitions,'SoundEffect_t','SOUND_EFFECT_TYPE_')}
    return helper,constants


def presentation_parameters(slot):
    donor=slot['source'];params=copy.deepcopy(slot['source_parameters']);helper,constants=sound_helper(donor)
    # These unknown globals are nil in the donor Lua registry. Keep original raw
    # declarations unchanged, while passing their actual nil to the helper probe.
    if params.get('type') in inline.UNDEFINED_NAMES:params.pop('type')
    capture=helper.capture(params,1,registered=False)
    if len(capture['actual_helper_call_variants'])!=1:raise ValueError('unexpected nondeterministic inline presentation')
    calls=capture['actual_helper_call_variants'][0];last={method:args for method,args in calls}
    def binding(method,default):
        symbol=(last.get(method) or [default])[0]
        if not isinstance(symbol,str) or symbol.lstrip('@') not in constants:raise ValueError('unresolved donor presentation '+str(symbol))
        name=symbol.lstrip('@');return {'source_symbol':name,'donor_numeric_id':constants[name],'native_asset_allocated':False}
    return {'impact':binding('setCombatEffect','@CONST_ME_NONE'),'projectile':binding('setCombatShootEffect','@CONST_ANI_NONE'),
            'cast_sound':binding('castSound','@SOUND_EFFECT_TYPE_SILENCE'),'impact_sound':binding('impactSound','@SOUND_EFFECT_TYPE_SILENCE'),
            'source_helper_sound_overwrite':'fallback_impact_written_to_castSound','cast_sound_write_sequence':[args[0].lstrip('@') for method,args in calls if method=='castSound'],
            'sound_playback_provider_qualified':False,'visual_playback_provider_qualified':False}


def build():
    output=[]
    for slot in cohort():
        original=inline.exact_cohort()[json.dumps(slot['slot_identity'],sort_keys=True)]
        for field in ['source','source_parameters','original_slot_sha256']:
            if slot[field]!=original[field]:raise ValueError('R51 original inline invariant differs: '+field)
        normalized=inline.build_inline(original);combat=normalized['normalized_combat'];donor=slot['source']
        proofs,scopes=helpers(donor)
        proofs=copy.deepcopy(proofs)+[{'source':donor,**original['monster_source'],'scope':'exact original inline MonsterSlot declaration'}]
        constructor={'combat_type':'COMBAT_UNDEFINEDDAMAGE','donor_combat_type_id':spell_scripts.enum_values(inline.read(donor,'src/creatures/creatures_definitions.hpp').decode(),'CombatType_t')['COMBAT_UNDEFINEDDAMAGE'],'origin':'ORIGIN_SPELL','aggressive':True,'blocked_by_armor':False,'blocked_by_shield':False,'use_weapon_charges':False,
                     'condition_type':'CONDITION_NONE','condition_appended':False,'dispel_type':'CONDITION_NONE','created_item_id':0,'chain_callback':False,'target_callback':False}
        signed={'minimum':combat['min_combat_value'],'maximum':combat['max_combat_value'],'distribution':'donor_truncated_normal','normal_mean':0.5,'normal_sigma':0.25,'reject_outside_unit_interval':True,'quantization':'minimum_plus_lround_of_normalized_draw_times_maximum_minus_minimum','equal_bounds_rule':'still_draw_then_return_bound',
                'positive_branch':'gain_target_health','negative_branch':'undefined_type_target_health_damage','zero_branch':'nonpositive_health_path_with_zero_base_value',
                'resource_destination':'health','valid_mana_drain_dispatch':False,'builtin_life_drain_type':False,'mana_shield_rule':'skip_when_current_health_damage_type_is_undefined_after_hooks','branch_selection':'current_signed_value_at_each_health_provider_entry',
                'post_draw_events_and_modifiers':'donor_healthchange_and_combat_providers','preserve_signed_bounds':True}
        parameters={'type_resolution':normalized['source_type_resolution'],'constructor':constructor,'signed_health':signed,'geometry':geometry(donor,combat),'presentation':presentation_parameters(slot),
                    'donor_intended_damage_type':'UNKNOWN','canonical_normalization':None,'source_type_repair_performed':False,'cast_return':'true_after_native_dispatch_when_combat_exists'}
        schedule={'kind':'donor_monster_attack_slot','interval_ms':combat['interval_ms'],'chance_percent':combat['chance_percent'],'range_tiles':combat['range_tiles'],'source_range_cap_tiles':22,
                  'melee':False,'source_group':slot['slot_identity']['group'],'attack_scheduler_owner':'donor_monster_attack_selection','timing_guard':'attackTicks_at_least_interval_and_modulo_less_than_tick_interval','range_metric':'chebyshev_xy','range_zero_unlimited':True,'target_required':combat['need_target']}
        output.append({'slot_identity':copy.deepcopy(slot['slot_identity']),'source':donor,'monster':slot['monster'],'original_slot_sha256':slot['original_slot_sha256'],
                       'source_parameters':copy.deepcopy(slot['source_parameters']),'controller':{'kind':'undefined_signed_health','parameters':parameters},'source_proofs':proofs,'helper_scope_proofs':scopes,
                       'target_schedule':schedule,'full_slot_projection_complete':True,'status':'CANDIDATE_SCHEMA_VALID','target_schema_family':'private_source_complete_monster_v2',**FLAGS})
    return output


def schema(rows):
    # Concrete closed domain alternatives, not an executable observation schema.
    controllers=[]
    for row in rows:
        branch=fixed(row['controller'])
        if branch not in controllers:controllers.append(branch)
    return {'$schema':'https://json-schema.org/draft/2020-12/schema','$id':'urn:oteryn:monster-inline-controller:1',
            'title':'Proposed undefined signed-health MonsterSlot controller contract', '$defs':{'controller':{'anyOf':controllers}},'$ref':'#/$defs/controller'}


def write(path,value):path.write_text(json.dumps(value,ensure_ascii=False,sort_keys=True,indent=2)+'\n')


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--out',type=Path,default=OUT);args=parser.parse_args();out=args.out;out.mkdir(parents=True,exist_ok=True)
    rows=build();definition=schema(rows);jsonschema.Draft202012Validator.check_schema(definition);validator=jsonschema.Draft202012Validator(definition)
    for row in rows:validator.validate(row['controller'])
    write(SCHEMA,definition);write(out/'monster-inline-controller.schema.json',definition)
    payload=canonical({'schema':'OTERYN_MONSTER_INLINE_SOURCE_COMPLETE/v1','records':rows,'record_count':10,**FLAGS})
    encoded=gzip.compress(payload,mtime=0);(out/'monster-inline-complete.json.gz').write_bytes(encoded)
    write(out/'projection-proof.json',{'schema':'OTERYN_MONSTER_INLINE_SOURCE_COMPLETE_PROOF/v1','records':10,'full_slot_candidates':10,'prior_inline_count':19,'excluded_existing_d25_count':9,
        'payload_sha256':sha(payload),'gzip_sha256':sha(encoded),'schema_sha256':sha(SCHEMA.read_bytes()),'schema_id':definition['$id'],'schema_ref':'monster-inline-controller.schema.json#/$defs/controller',
        'producer_sha256':sha(Path(__file__).read_bytes()),'input_proofs':{str(path.relative_to(ROOT)):sha(path.read_bytes()) for path in [R51,R54,inline.LINKS]},
        'source_pins':inline.PINS,'source_proofs':{donor:helpers(donor)[0] for donor in ['canary','crystal']},'existing_d25_normalizations_changed':False,'donor_intent_invented':False,**FLAGS})
    write(out/'package-manifest.json',{'schema':'OTERYN_MONSTER_INLINE_SOURCE_COMPLETE_PACKAGE/v1','files':{path.relative_to(out).as_posix():sha(path.read_bytes()) for path in sorted(out.rglob('*')) if path.is_file() and path.name!='package-manifest.json'}})
    print(json.dumps({'records':10,'full_slot_candidates':10,'runtime_activation':False}))


if __name__=='__main__':main()
