"""Generate the closed draft-2020-12 Wheel authoring schema; no runtime admission."""
import json
import sys
from pathlib import Path
from wheel_authoring import read, parameter_value_schema, COUNT_UNITS, BOUNDED_PERCENT_KINDS
ROOT = Path(__file__).resolve().parent
VOCATIONS = ['knight', 'paladin', 'sorcerer', 'druid', 'monk']
DOMAINS = ['green', 'red', 'blue', 'purple']
def obj(properties, required=None):
    return {'type':'object','additionalProperties':False,'properties':properties,'required':list(properties) if required is None else required}
def arr(item, lo=0, hi=None, unique=False):
    s={'type':'array','items':item,'minItems':lo}
    if hi is not None:s['maxItems']=hi
    if unique:s['uniqueItems']=True
    return s
def enum(values):return {'enum':values}
def ref(name):return {'$ref':'#/$defs/'+name}
def integer(lo,hi=None):
    s={'type':'integer','minimum':lo}
    if hi is not None:s['maximum']=hi
    return s
parameters=read(ROOT/'samples/source-parameters.json')
text={'type':'string','minLength':1,'maxLength':20000}
number={'type':'number','minimum':0}
slot_id=integer(1,36)
key={'type':'string','pattern':'^[a-z][a-z0-9_]*$','maxLength':128}
icon=obj({'sprite':enum(['dedication','conviction','revelation','basic_mod','supreme_mod']),'source_index':integer(0,255),'verification':{'const':'SOURCE_MAPPING_ONLY'},'layout':{'const':'horizontal_square_cells'},'asset_url':{'type':'string','pattern':'^https://static\\.tibia\\.com/images/'}})
effect=obj({'kind':enum(['base_damage_bonus','base_healing_bonus','cooldown_reduction','secondary_cooldown_reduction','mana_cost_reduction','life_leech','critical_hit_chance','critical_extra_damage','additional_targets','range_increase','next_attack_damage_reduction','duration_increase']),'value':number,'unit':enum(['percent_points','seconds','mana','targets','tiles'])})
effect['allOf']=[{'if':{'properties':{'unit':enum(list(COUNT_UNITS))}},'then':{'properties':{'value':{'type':'integer'}}}},
    {'if':{'properties':{'kind':enum(list(BOUNDED_PERCENT_KINDS))}},'then':{'properties':{'value':{'maximum':100}}}}]
stage=obj({'stage':integer(1,2),'reference_text':text,'numeric_effects':arr(ref('augment_effect')),'unresolved_parameters':arr(text,0,0,True),'area_reference':{'type':['string','null'],'enum':[None,'five_squares_front_row','AREA_WAVE7','radius_4','AREA_GREATER_FLURRY_OF_BLOWS']}})
conviction=obj({'key':key,'name':text,'category':enum(['unique','skill_bonus','leech','augmentation','vessel_resonance','other']),'source_info_id':integer(0,255),'native_value':number,'unit':enum(['flat','percent_points','native','stages']),'reference_description':{'type':'string','maxLength':20000},'augment_targets':arr(text,0,None,True),'reference_hypotheses':arr(ref('augment_hypothesis')),'unique_parameters':{'oneOf':[{'type':'null'},ref('unique_parameters')]},'augment_stages':arr(ref('augment_stage'),0,2),'icon':ref('icon')})
dedication=obj({'stat':enum(['max_health','max_mana','capacity','mitigation_multiplier']),'value_per_point':number,'unit':enum(['flat','percent_points']),'stacking':enum(['add_to_maximum','increase_base_mitigation_multiplicatively'])})
slot=obj({'state_slot':slot_id,'dedication':arr(ref('dedication'),1,2),'dedication_icon':ref('icon'),'conviction':ref('conviction')})
revelation_stage=obj({'stage':integer(1,3),'minimum_domain_points':integer(1,1000),'reference_description':text,'numeric_effects':arr(ref('revelation_effect'),1)})
revelation=obj({'domain':enum(DOMAINS),'key':key,'name':text,'source_info_id':integer(0,255),'behavior_rules':arr(enum(sorted({b for bs in parameters['revelation_behaviors'].values() for b in bs})),1,None,True),'area_reference':{'enum':[None]+sorted({v for v in parameters['revelation_area_references'].values() if v})},'stage_zero_description':text,'stages':arr(ref('revelation_stage'),3,3),'parameter_state':{'const':'REFERENCE_PARAMETERS_CAPTURED'},'icon':ref('icon')})
vocation=obj({'gem_family':obj({'family':enum(['guardian','marksman','sage','mystic','spiritualist']),'items':obj({q:{'type':'string','pattern':'^oteryn:item.tibia.i[0-9]+$'} for q in ['lesser','regular','greater']})}),'resonance_slots':obj({d:arr(slot_id,3,3,True) for d in DOMAINS}),'slots':arr(ref('slot'),36,36),'revelations':arr(ref('revelation'),4,4),'gem_names':obj({q:text for q in ['lesser','regular','greater']}),'basic_mods_position_1':arr(integer(0,255),1,255,True),'basic_mods_position_2':arr(integer(0,255),1,255,True),'supreme_mods':arr(integer(0,255),1,255,True)})
topology=obj({'state_slot':slot_id,'domain':enum(DOMAINS),'capacity':enum([50,75,100,150,200]),'minimum_available_points':integer(0,4000),'unlock_from_any_full_slot':arr(slot_id,0,8,True),'source':obj({'canary_enum':{'type':'string','pattern':'^SLOT_(GREEN|RED|BLUE|PURPLE)_[A-Z0-9_]+$'},'canary_slot_id':slot_id,'tibiapal_tile':{'type':'string','pattern':'^Q(TL|TR|BL|BR)[0-8]$'}})})
modgrade=obj({'grade':integer(0,3),'reference_text':text,'numeric_effects':arr(ref('supreme_effect'),1,2)})
basic_effect=obj({'source_effect_id':integer(0,255),'name':text,'unit':enum(['flat','percent_points']),'values_by_vocation':obj({v:arr({'type':'number'},4,4) for v in VOCATIONS})})
basic_mod=obj({'source_id':integer(0,255),'effects':arr(ref('basic_effect'),1,3),'icon':ref('icon')})
supreme_mod=obj({'source_id':integer(0,255),'name':text,'summary_name':text,'format':text,'grades':arr(ref('mod_grade'),4,4),'reference_summary':{'type':'string','maxLength':20000},'icon':ref('icon')})
gem_quality=obj({'quality':enum(['lesser','regular','greater']),'basic_mod_count':integer(1,2),'supreme_mod_count':integer(0,1),'matching_vessel_stage':integer(1,3),'matching_damage_healing_bonus':integer(1,2)})
cost=obj({'gold':integer(0),'fragments':integer(0)})
grade_cost=obj({'target_grade':integer(1,3),'basic':ref('cost'),'supreme':ref('cost')})
source=obj({'id':key,'repository':{'type':'string','pattern':'^https://github.com/'},'commit':{'type':'string','pattern':'^[a-f0-9]{40}$'}})
gem_revision=obj({'kind':enum(['declared_compatible','staged_migration']),
    'reference':{'type':'string','pattern':r'^samples/gem-revisions/[a-z0-9][a-z0-9_-]*\.json$'},
    'sha256':{'type':'string','pattern':'^[a-f0-9]{64}$'}})
release={'oneOf':[obj({'kind':{'const':'initial'},'predecessor':{'type':'null'}}),
    obj({'kind':enum(['value_only','wheel_reset']),'predecessor':text,'gem_revision':gem_revision},['kind','predecessor'])]}
parameter_units={e['kind']:e['unit'] for effects in parameters['revelations'].values() for e in effects}
revelation_effect={'oneOf':[obj({'kind':{'const':k},'value':parameter_value_schema(k,u),'unit':{'const':u}}) for k,u in parameter_units.items()]}
supreme_units={'dodge':'percent_points','critical_extra_damage':'percent_points','life_leech':'percent_points','mana_leech':'percent_points','base_damage_bonus':'percent_points','base_healing_bonus':'percent_points','cooldown_reduction':'seconds','momentum_chance':'percent_points','revelation_mastery_points':'points'}
supreme_effect={'oneOf':[obj({'kind':{'const':k},'value':parameter_value_schema(k,u),'unit':{'const':u},'target':text}) for k,u in supreme_units.items()]}
# Candidate Atelier policy: source values remain labelled hypotheses or official facts.
atelier=obj({'clockwise_domains':arr(enum(DOMAINS),4,4,True),'basic_pair_compatibility':{'const':'different_source_mod_ids'},'effective_grade_order':{'const':['basic_1','basic_2','supreme']},'effective_grade_rule':{'const':'minimum_of_self_and_present_preceding_mod_grades'},'grade_iv_promotion_points_per_mod_type':integer(0),'fees':obj({action:obj({q:integer(0) for q in ['lesser','regular','greater']}) for action in ['reveal','switch_domain']}),'fragment_items':obj({k:{'type':'string','pattern':'^oteryn:item.tibia.i[0-9]+$'} for k in ['basic','supreme']}),'fragment_yields':obj({q:obj({'fragment':enum(['basic','supreme']),'unrevealed':arr(integer(0),2,2),'revealed':arr(integer(0),2,2)}) for q in ['lesser','regular','greater']}),'operation_policy':ref('operation_policy'),'yield_evidence':text,'fee_evidence':text,'grade_scope':{'const':'character_mod_type'},'resonance_activation_order':{'const':['basic_1','basic_2','supreme']}})
correction=obj({'key':key,'parameter':key,'planner_parameter':key,'stage':integer(1,3),'planner_value':number,'selected_value':number,'evidence':text})
augment_hypothesis={'oneOf':[obj({'stage':integer(1,2),'kind':{'const':k},'value':number,'unit':{'const':u},'evidence':text}) for k,u in [('secondary_cooldown_reduction','seconds'),('range_increase','tiles')]]}
unique_units={e['kind']:e['unit'] for p in parameters['unique_conviction'].values() for e in p['numeric_effects']}
unique_effect={'oneOf':[obj({'kind':{'const':k},'value':parameter_value_schema(k,u),'unit':{'const':u}}) for k,u in unique_units.items()]}
unique_parameters=obj({'numeric_effects':arr(ref('unique_effect')),'behaviors':arr(enum(sorted({b for p in parameters['unique_conviction'].values() for b in p['behaviors']})),1,None,True),'targets':arr(text,0,None,True)})
def literal_shape(value):
    if isinstance(value,dict):return obj({k:literal_shape(v) for k,v in value.items()})
    if isinstance(value,bool):return {'type':'boolean'}
    if isinstance(value,int):return integer(0)
    if isinstance(value,float):return number
    if isinstance(value,str):return text
    if isinstance(value,list):
        if not value:return arr(text)
        return arr(literal_shape(value[0]),len(value),len(value))
    raise TypeError(value)
progression=literal_shape(parameters['progression'])
operation_policy=literal_shape(parameters['gems']['operation_policy'])
# These accepted WHEEL-GEM-0 invariants cannot be changed by editing a capture.
operation_contract={'revealed_tradeable':False,'reveal_requires_matching_vocation':True,
    'grade_min':0,'grade_max':3,'grade_decrease_allowed':False,
    'vessel_requires_matching_domain':True,'gem_can_occupy_only_one_vessel':True,
    'initial_gems_once_per_character':True,'initial_gems_revealed':True}
for field,value in parameters['gems']['operation_policy'].items():
    if field in operation_contract and value!=operation_contract[field]:raise ValueError('ATELIER_CONTRACT: '+field)
    if field not in ('vendor_reference_prices','vendor_buy_unrevealed_gem_prices'):
        operation_policy['properties'][field]={'const':value}
icon_evidence=literal_shape(parameters['icon_evidence'])
loot_reference=literal_shape(parameters['gems']['loot_reference'])
loot_reference['properties']['roll_denominator']=integer(1)
root=obj({'schema':{'const':'OTERYN_WHEEL_AUTHORING_CANDIDATE/v1'},'revision':{'type':'string','minLength':1,'maxLength':128},'release':release,'runtime_admitted':{'const':False},'sources':arr(ref('source'),1,None),'input_digests':obj({f:{'type':'string','pattern':'^[a-f0-9]{64}$'} for f in ['source-wheel-reference.json','source-graph.json','source-parameters.json']}),'progression':ref('progression'),'icon_evidence':ref('icon_evidence'),'topology':arr(ref('topology'),36,36),'vocations':obj({v:ref('vocation') for v in VOCATIONS}),'gems':obj({'excluded_empty_basic_mod_ids':arr(integer(0,255),0,255,True),'qualities':arr(ref('gem_quality'),3,3),'basic_mods':arr(ref('basic_mod'),1,255),'supreme_mods':arr(ref('supreme_mod'),1,255),'grade_costs':arr(ref('grade_cost'),3,3),'atelier':ref('atelier'),'loot_reference':ref('loot_reference'),'reference_corrections':arr(ref('correction')),'initial_gems':obj({'count':{'const':8},'composition':{'const':'one_lesser_and_one_regular_per_domain'}}),'parameter_state':{'const':'REFERENCE_CATALOGUE_NOT_RUNTIME_ADMITTED'}}),'verification':obj({'planner_unlock_states_checked':integer(1),'planner_unlock_mismatches':{'const':0},'legal_allocation_snapshots':integer(1),'live_website_verified':{'const':parameters['live_source_verification']['tibiapal_content_verified']},'wiki_verified':{'const':parameters['live_source_verification']['requested_fandom_verified']},'blockers':arr(text,1,None,True)})})
root.update({'$schema':'https://json-schema.org/draft/2020-12/schema','title':'Wheel of Destiny authoring candidate v1','$defs':{k:v for k,v in locals().copy().items() if k in ['icon','effect','stage','conviction','dedication','slot','revelation_stage','revelation','vocation','topology','modgrade','basic_effect','basic_mod','supreme_mod','gem_quality','cost','grade_cost','source','revelation_effect','supreme_effect','atelier','correction','unique_effect','unique_parameters','progression','operation_policy','icon_evidence','loot_reference','augment_hypothesis']}})
root['$defs']['augment_effect']=root['$defs'].pop('effect');root['$defs']['augment_stage']=root['$defs'].pop('stage');root['$defs']['mod_grade']=root['$defs'].pop('modgrade')
def output(filename,value):
    path=ROOT/filename;rendered=json.dumps(value,indent=2,allow_nan=False)+'\n'
    if '--check' in sys.argv:
        if path.read_text()!=rendered:raise ValueError('SCHEMA_REBUILD_DRIFT: '+filename)
    else:path.write_text(rendered)
output('wheel.schema.json',root)

evidence=literal_shape(read(ROOT/'samples/verification-evidence.json'))
evidence['properties']['schema']={'const':'OTERYN_WHEEL_REFERENCE_VERIFICATION/v1'}
evidence['properties']['candidate_sha256']={'type':'string','pattern':'^[a-f0-9]{64}$'}
evidence.update({'$schema':'https://json-schema.org/draft/2020-12/schema','title':'Wheel reference verification evidence'})
output('verification.schema.json',evidence)
