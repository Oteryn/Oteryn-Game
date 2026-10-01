"""Build/validate a reference-only Wheel candidate. Never writes runtime rulesets."""
from __future__ import annotations
import argparse
import hashlib
import html
import json
import math
import re
from pathlib import Path
from jsonschema import Draft202012Validator
ROOT = Path(__file__).resolve().parent
VOCATIONS = ('knight', 'paladin', 'sorcerer', 'druid', 'monk')
DOMAINS = {'TL':'green','TR':'red','BL':'blue','BR':'purple'}
CATEGORY = {'Unique':'unique','SkillBonus':'skill_bonus','Leech':'leech','Augmentation':'augmentation','VesselResonance':'vessel_resonance','Other':'other'}
STATS = {'Hit Points':'max_health','Mana':'max_mana','Capacity':'capacity','Mitigation Multiplier':'mitigation_multiplier'}
def finite_json(value):
    if isinstance(value,float) and not math.isfinite(value):raise ValueError('NON_FINITE_NUMBER')
    if isinstance(value,dict):
        for child in value.values():finite_json(child)
    elif isinstance(value,list):
        for child in value:finite_json(child)
def read(path):
    def reject_constant(value):raise ValueError('NON_FINITE_NUMBER: '+value)
    value=json.loads(Path(path).read_text(),parse_constant=reject_constant)
    finite_json(value)
    return value
def clean(text): return html.unescape(re.sub(r'<[^>]*>', ' ', text)).strip()
def name(text): return clean(text).split('|')[0]
def slug(text): return re.sub(r'[^a-z0-9]+','_', name(text).lower()).strip('_')
def icon(sprite, index): return {'sprite':sprite,'source_index':index,'verification':'SOURCE_MAPPING_ONLY','layout':'horizontal_square_cells','asset_url':{'dedication':'https://static.tibia.com/images/community/wheelofdestiny/icons-skillwheel-smallperks.png','conviction':'https://static.tibia.com/images/community/wheelofdestiny/icons-skillwheel-mediumperks.png','revelation':'https://static.tibia.com/images/community/wheelofdestiny/icons-skillwheel-largeperks.png','basic_mod':'https://static.tibia.com/images/global/common/skillwheel/icons-skillwheel-basicmods.png','supreme_mod':'https://static.tibia.com/images/global/common/skillwheel/icons-skillwheel-suprememods.png'}[sprite]}
def number(text):
    match = re.search(r'[+-]?\d+(?:\.\d+)?', text)
    if not match: raise ValueError('No numeric value in '+text)
    return float(match[0])
def augment_stage(stage, description):
    text = clean(description)
    effects = []
    patterns = [
        (r'\+([\d.]+)% Base Damage', 'base_damage_bonus','percent_points'),
        (r'\+([\d.]+)% Base Healing', 'base_healing_bonus','percent_points'),
        (r'-([\d.]+)s Cooldown', 'cooldown_reduction','seconds'),
        (r'secondary group cooldown -([\d.]+)s','secondary_cooldown_reduction','seconds'),
        (r'-([\d.]+) Mana Cost','mana_cost_reduction','mana'),
        (r'Adds ([\d.]+)% life leech', 'life_leech','percent_points'),
        (r'Adds ([\d.]+)% critical hit chance','critical_hit_chance','percent_points'),
        (r'Adds ([\d.]+)% critical extra damage','critical_extra_damage','percent_points'),
        (r'Jumps to \+([\d.]+) additional target','additional_targets','targets'),
        (r'Range increased by ([\d.]+)','range_increase','tiles'),
        (r'^-([\d.]+)% damage from the next hostile auto attack','next_attack_damage_reduction','percent_points'),
    ]
    for pattern, kind, unit in patterns:
        match = re.search(pattern,text,re.I)
        if match:effects.append({'kind':kind,'value':float(match[1]),'unit':unit})
    unresolved=[]
    if 'shape' in text.lower() or 'area' in text.lower():unresolved.append('Exact affected-tile pattern requires the Spell/WorldQuery owner.')
    if 'duration increased' in text.lower():unresolved.append('Exact duration is not stated in this planner description.')
    if not effects and not unresolved:raise ValueError('Unmapped augment: '+text)
    return {'stage':stage,'reference_text':text,'numeric_effects':effects,'unresolved_parameters':unresolved}
def supreme_effects(description, target):
    kinds={'Dodge':('dodge','percent_points'),'Critical Extra Damage':('critical_extra_damage','percent_points'),'Life Leech':('life_leech','percent_points'),'Mana Leech':('mana_leech','percent_points'),'Base Damage':('base_damage_bonus','percent_points'),'Base Healing':('base_healing_bonus','percent_points'),'Cooldown':('cooldown_reduction','seconds'),'Momentum':('momentum_chance','percent_points')}
    result=[]
    for term in description.split(','):
        match=re.fullmatch(r'\s*([+-]?\d+(?:\.\d+)?)(%|s)?\s+(.+?)\s*',term)
        if not match:raise ValueError('Unmapped supreme effect: '+term)
        value=float(match[1]);label=match[3]
        if label in kinds:kind,unit=kinds[label]
        else:kind,unit='revelation_mastery_points','points'
        if kind=='revelation_mastery_points' and (match[2] or not target.startswith('Revelation Mastery')):raise ValueError('Unknown supreme target: '+label)
        result.append({'kind':kind,'value':abs(value) if kind=='cooldown_reduction' else value,'unit':unit,'target':label if kind=='revelation_mastery_points' else name(target)})
    return result
def build():
    raw=read(ROOT/'samples/source-wheel-reference.json');graph=read(ROOT/'samples/source-graph.json'); parameters=read(ROOT/'samples/source-parameters.json')
    mapping={r['source_name']:r['state_slot'] for r in graph['rules']}
    reference_slots=raw['vocations']['knight']['slots']
    topology=[]
    for rule, slot in zip(graph['rules'],reference_slots,strict=True):
        domain=rule['source_name'].split('_')[1].lower()
        if DOMAINS[slot['tile'][1:3]]!=domain:raise ValueError('Domain mapping conflict')
        capacity=int(rule['source_name'].split('_')[-1])
        if capacity!=slot['capacity']:raise ValueError('Capacity mapping conflict')
        topology.append({'state_slot':rule['state_slot'],'domain':domain,'capacity':capacity,'minimum_available_points':rule['minimum_available_points'],'unlock_from_any_full_slot':[mapping[n] for n in rule['neighbors']],'source':{'canary_enum':rule['source_name'],'canary_slot_id':rule['state_slot'],'tibiapal_tile':slot['tile']}})
    vocations={}
    for voc in VOCATIONS:
        original=raw['vocations'][voc];slots=[]
        for top, slot in zip(topology,original['slots'],strict=True):
            if slot['tile']!=top['source']['tibiapal_tile']:raise ValueError('Vocation mapping conflict')
            dedication=[]
            for eff in slot['dedication']['effects_per_point']:
                stat=STATS[name(eff['stat'])]
                dedication.append({'stat':stat,'value_per_point':graph['dedication_mitigation']['percent_per_point'] if stat=='mitigation_multiplier' else eff['value'],'unit':'percent_points' if eff['unit']=='percent' else 'flat','stacking':'increase_base_mitigation_multiplicatively' if stat=='mitigation_multiplier' else 'add_to_maximum'})
            c=slot['conviction'];info=c['info'];category=CATEGORY[c['category']]
            value=c['value'];unit='native'
            if category in ('augmentation','vessel_resonance'):value=1;unit='stages'
            elif info['FormatType']=='PlusPercentWithTwoFloatingpoints':value/=100;unit='percent_points'
            elif info['FormatType']=='PlusFullPercent':unit='percent_points'
            elif info['FormatType']=='PlusInteger':unit='flat'
            stages=[augment_stage(i,info[f'Aug{i}Info']) for i in (1,2)] if category=='augmentation' else []
            conviction={'key':slug(info['Name']),'name':name(info['Name']),'category':category,'source_info_id':c['id'],'native_value':value,'unit':unit,'reference_description':clean(info.get('LongInfo','')),'augment_targets':parameters['augment_targets'].get(slug(info['Name']),[name(info['Name']).removeprefix('Augmented ')] if category=='augmentation' else []),'reference_hypotheses':parameters['augment_hypotheses'].get(slug(info['Name']),[]),'unique_parameters':parameters['unique_conviction'].get(slug(info['Name'])),'augment_stages':stages,'icon':icon('conviction',c['id'])}
            for stage in conviction['augment_stages']:
                area = parameters['area_references'].get(conviction['key'])
                stage['area_reference'] = area['reference'] if area and area['stage']==stage['stage'] else None
                extra = parameters['augment_parameters'].get(conviction['key'])
                if extra and extra['stage']==stage['stage']:
                    stage['numeric_effects'].append({k:extra[k] for k in ('kind','value','unit')})
                for correction in parameters['corrections']:
                    if correction['key']==conviction['key'] and correction['stage']==stage['stage']:
                        if correction['planner_parameter']!=correction['parameter']:raise ValueError('CROSS_KIND_CORRECTION')
                        effect=next(e for e in stage['numeric_effects'] if e['kind']==correction['planner_parameter'])
                        effect['value']=correction['selected_value']
                for hypothesis in conviction['reference_hypotheses']:
                    if hypothesis['stage']==stage['stage']:
                        stage['numeric_effects']=[e for e in stage['numeric_effects'] if e['kind']!=hypothesis['kind']]
                stage['unresolved_parameters']=[reason for reason in stage['unresolved_parameters']
                    if not (('affected-tile' in reason and stage['area_reference']) or
                            ('duration' in reason and extra and extra['stage']==stage['stage'] and extra['kind']=='duration_increase'))]
            slots.append({'state_slot':top['state_slot'],'dedication':dedication,'dedication_icon':icon('dedication',slot['dedication']['id']),'conviction':conviction})
        revelations=[]
        for rev in original['revelations']:
            info=rev['info'];descriptions=info['LongInfoPerLevel']
            revelations.append({'domain':DOMAINS[rev['quarter']],'key':slug(info['Name']),'name':name(info['Name']),'source_info_id':rev['id'],'behavior_rules':parameters['revelation_behaviors'][slug(info['Name'])],'area_reference':parameters['revelation_area_references'][slug(info['Name'])],'stage_zero_description':clean(descriptions['0']),'stages':[{'stage':i,'minimum_domain_points':threshold,'reference_description':clean(descriptions[str(i)]),'numeric_effects':[{'kind':e['kind'],'value':e['values'][i-1],'unit':e['unit']} for e in parameters['revelations'][slug(info['Name'])]]} for i,threshold in [(1,250),(2,500),(3,1000)]],'parameter_state':'REFERENCE_PARAMETERS_CAPTURED','icon':icon('revelation',rev['id'])})
        gems=original['gems'];vessels=raw['gem_library']['vessels']
        vocations[voc]={'gem_family':parameters['gem_families'][voc],'resonance_slots':{domain:[top['state_slot'] for top,entry in zip(topology,slots,strict=True) if top['domain']==domain and entry['conviction']['category']=='vessel_resonance'] for domain in DOMAINS.values()},'slots':slots,'revelations':revelations,'gem_names':{q:vessels['GemNames'][str(i)][voc] for i,q in enumerate(('lesser','regular','greater'))},'basic_mods_position_1':[m['id'] for m in gems['basic_mods_position_1']],'basic_mods_position_2':[m['id'] for m in gems['basic_mods_position_2']],'supreme_mods':gems['supreme_mod_ids']}
    lib=raw['gem_library'];basic=[]
    for identifier, effects in sorted(lib['basic_mod_config'].items(),key=lambda x:int(x[0])):
        if not effects:continue
        converted=[]
        for effect in effects:
            info=lib['basic_mod_effects'][effect['EffectId']]
            converted.append({'source_effect_id':int(effect['EffectId']),'name':name(info['Name']),'unit':'flat' if info['FormatType']=='PlusInteger' else 'percent_points','values_by_vocation':{v:[number(effect[v][str(g)]) for g in range(4)] for v in VOCATIONS}})
        basic.append({'source_id':int(identifier),'effects':converted,'icon':icon('basic_mod',int(identifier))})
    supreme=[]
    for identifier, info in lib['supreme_mods'].items():
        if not identifier.isdigit():continue
        supreme.append({'source_id':int(identifier),'name':name(info['Name']) or name(info['NameSummary']),'summary_name':name(info['NameSummary']),'format':info['FormatType'],'grades':[{'grade':g,'reference_text':clean(info['EffectInfo'][str(g)]),'numeric_effects':supreme_effects(clean(info['EffectInfo'][str(g)]),info['NameSummary'])} for g in range(4)],'reference_summary':clean(info['EffectInfoSummary']),'icon':icon('supreme_mod',int(identifier))})
    return {'schema':'OTERYN_WHEEL_AUTHORING_CANDIDATE/v1','revision':'wheel-authoring-candidate-r1','release':{'kind':'initial','predecessor':None},'runtime_admitted':False,'sources':[{'id':'tibiapal','repository':'https://github.com/PawelKusnierek/TibiaPal','commit':raw['source']['commit']},{'id':'canary','repository':'https://github.com/opentibiabr/canary','commit':graph['source_commit']},{'id':'crystal_summer_update','repository':'https://github.com/zimbadev/crystalserver','commit':'00ce02a57ca5a12e48f32a3476e37471167e4c3f'}],'input_digests':{f:hashlib.sha256((ROOT/'samples'/f).read_bytes()).hexdigest() for f in ['source-wheel-reference.json','source-graph.json','source-parameters.json']},'progression':parameters['progression'],'icon_evidence':parameters['icon_evidence'],'topology':topology,'vocations':vocations,'gems':{'excluded_empty_basic_mod_ids':[int(k) for k,v in lib['basic_mod_config'].items() if not v],'qualities':[{'quality':q,'basic_mod_count':b,'supreme_mod_count':s,'matching_vessel_stage':v,'matching_damage_healing_bonus':bonus} for q,b,s,v,bonus in [('lesser',1,0,1,1),('regular',2,0,2,1),('greater',2,1,3,2)]],'basic_mods':basic,'supreme_mods':supreme,'grade_costs':[{'target_grade':g,'basic':{'gold':bg,'fragments':f},'supreme':{'gold':sg,'fragments':f}} for g,bg,sg,f in [(1,2000000,5000000,5),(2,5000000,12000000,15),(3,30000000,75000000,30)]],'atelier':{k:v for k,v in parameters['gems'].items() if k!='loot_reference'},'loot_reference':parameters['gems']['loot_reference'],'reference_corrections':parameters['corrections'],'initial_gems':{'count':8,'composition':'one_lesser_and_one_regular_per_domain'},'parameter_state':'REFERENCE_CATALOGUE_NOT_RUNTIME_ADMITTED'},'verification':{'planner_unlock_states_checked':1080,'planner_unlock_mismatches':0,'legal_allocation_snapshots':180,'live_website_verified':parameters['live_source_verification']['tibiapal_content_verified'],'wiki_verified':parameters['live_source_verification']['requested_fandom_verified'],'blockers':['Formal admission requires the content owner review; this candidate changes no runtime ruleset.','Resolve dedication mitigation versus resistance wording in the owning decision.','Bind reference parameters and area names to admitted runtime effect and Spell owners.','Verify icon appearance and client-asset mappings.','Reconcile the remaining target-version perk conflicts and requested Fandom pages; see samples/live-source-audit.json.']}}
def validate_gem(candidate, vocation, quality, basic_1, basic_2=None, supreme=None):
    if vocation not in VOCATIONS:raise ValueError('UNKNOWN_VOCATION')
    if quality not in ('lesser','regular','greater'):raise ValueError('UNKNOWN_GEM_QUALITY')
    data=candidate['vocations'][vocation]
    if type(basic_1) is not int or basic_1 not in data['basic_mods_position_1']:raise ValueError('BASIC_1_NOT_ALLOWED')
    if quality=='lesser':
        if basic_2 is not None or supreme is not None:raise ValueError('GEM_MOD_SHAPE')
    else:
        if type(basic_2) is not int or basic_2 not in data['basic_mods_position_2']:raise ValueError('BASIC_2_NOT_ALLOWED')
        if basic_1==basic_2:raise ValueError('DUPLICATE_BASIC_MOD')
        if quality=='greater':
            if type(supreme) is not int or supreme not in data['supreme_mods']:raise ValueError('SUPREME_NOT_ALLOWED')
        elif supreme is not None:raise ValueError('GEM_MOD_SHAPE')

def effective_gem_grades(grades):
    if not 1<=len(grades)<=3 or any(type(g) is not int or not 0<=g<=3 for g in grades):raise ValueError('INVALID_GEM_GRADES')
    return [min(grades[:i+1]) for i in range(len(grades))]

def validate_allocation(candidate, voc, points, available_points):
    if voc not in VOCATIONS:raise ValueError('UNKNOWN_VOCATION')
    if type(available_points) is not int or available_points<0:raise ValueError('INVALID_POINT_BUDGET')
    if len(points)!=36 or any(type(x) is not int or x<0 for x in points):raise ValueError('INVALID_POINT_VECTOR')
    topology=candidate['topology'];by_id={s['state_slot']:s for s in topology}
    if sum(points)>available_points:raise ValueError('POINT_BUDGET_EXCEEDED')
    for identifier, points_in_slot in enumerate(points,1):
        slot=by_id[identifier]
        if points_in_slot>slot['capacity']:raise ValueError('SLOT_CAPACITY_EXCEEDED')
        if points_in_slot and available_points<slot['minimum_available_points']:raise ValueError('MINIMUM_POINTS_NOT_MET')
    # Each allocated slice must have a path from an inner slice through full slices.
    reached={i for i,s in by_id.items() if not s['unlock_from_any_full_slot']}
    while True:
        added={i for i,s in by_id.items() if i not in reached and any(n in reached and points[n-1]==by_id[n]['capacity'] for n in s['unlock_from_any_full_slot'])}
        if not added:break
        reached|=added
    if any(points[i-1]>0 and i not in reached for i in by_id):raise ValueError('DISCONNECTED_ALLOCATION')
def structure(candidate):
    """Only numeric values may change in a value-only successor."""
    def scrub(value, field=None):
        if field in ('value', 'value_per_point', 'native_value', 'gold', 'fragments'):
            return 'NUMERIC_VALUE'
        if field == 'values_by_vocation':
            return {v:len(xs) for v,xs in value.items()}
        if field in ('reference_text','reference_description','stage_zero_description','reference_summary'):
            return re.sub(r'[+-]?\d+(?:\.\d+)?', 'NUMERIC_VALUE', value)
        if isinstance(value,dict):return {k:scrub(v,k) for k,v in value.items()}
        if isinstance(value,list):return [scrub(v) for v in value]
        return value
    return scrub({k:candidate[k] for k in ['progression','topology','vocations','gems']})
def validate(candidate, previous=None):
    finite_json(candidate)
    schema=read(ROOT/'wheel.schema.json');Draft202012Validator.check_schema(schema);Draft202012Validator(schema).validate(candidate)
    def require(condition, code):
        if not condition:raise ValueError(code)
    release=candidate['release']
    if release['kind']=='initial':require(previous is None,'INITIAL_WITH_PREDECESSOR')
    else:
        require(previous is not None,'PREDECESSOR_REQUIRED');require(release['predecessor']==previous['revision'] and candidate['revision']!=previous['revision'],'REVISION_CHAIN')
        if release['kind']=='value_only':require(structure(candidate)==structure(previous),'VALUE_ONLY_STRUCTURE_CHANGED')
    for filename,digest in candidate['input_digests'].items():
        require(hashlib.sha256((ROOT/'samples'/filename).read_bytes()).hexdigest()==digest,'INPUT_DIGEST_MISMATCH')
    topology=candidate['topology'];require([s['state_slot'] for s in topology]==list(range(1,37)),'SLOT_IDENTITIES')
    require(len({s['source']['tibiapal_tile'] for s in topology})==36,'PLANNER_TILE_IDENTITIES')
    for s in topology:
        require(s['state_slot']==s['source']['canary_slot_id'],'SLOT_CROSSWALK')
        require(s['state_slot'] not in s['unlock_from_any_full_slot'],'SELF_DEPENDENCY')
        require(DOMAINS[s['source']['tibiapal_tile'][1:3]]==s['domain'],'DOMAIN_CROSSWALK')
    require(len([s for s in topology if not s['unlock_from_any_full_slot']])==4,'ROOT_COUNT')
    reached={s['state_slot'] for s in topology if not s['unlock_from_any_full_slot']}
    while True:
        added={s['state_slot'] for s in topology if any(i in reached for i in s['unlock_from_any_full_slot'])}-reached
        if not added:break
        reached|=added
    require(len(reached)==36,'UNREACHABLE_TOPOLOGY')
    for domain in DOMAINS.values():
        group=[s for s in topology if s['domain']==domain];require(len(group)==9 and sum(s['capacity'] for s in group)==1000,'DOMAIN_CAPACITY')
    reference_parameters=read(ROOT/'samples/source-parameters.json')
    for v in VOCATIONS:
        data=candidate['vocations'][v];require([s['state_slot'] for s in data['slots']]==list(range(1,37)),'VOCATION_SLOT_IDENTITIES')
        require(data['gem_family']==reference_parameters['gem_families'][v],'GEM_FAMILY_BINDING')
        for domain in DOMAINS.values():
            expected=[t['state_slot'] for t,e in zip(topology,data['slots'],strict=True) if t['domain']==domain and e['conviction']['category']=='vessel_resonance']
            require(data['resonance_slots'][domain]==expected and len(expected)==3,'RESONANCE_SLOTS')
        for s in data['slots']:
            require(len({e['stat'] for e in s['dedication']})==len(s['dedication']),'DUPLICATE_DEDICATION')
            for effect in s['dedication']:
                is_mitigation=effect['stat']=='mitigation_multiplier'
                require(effect['unit']==('percent_points' if is_mitigation else 'flat'),'DEDICATION_UNIT')
                require(effect['stacking']==('increase_base_mitigation_multiplicatively' if is_mitigation else 'add_to_maximum'),'DEDICATION_STACKING')
            require(s['dedication_icon']['sprite']=='dedication','DEDICATION_ICON_SPRITE')
            require(s['dedication_icon']==icon('dedication',s['dedication_icon']['source_index']),'ICON_ASSET_BINDING')
            c=s['conviction'];require(c['icon']['source_index']==c['source_info_id'],'CONVICTION_ICON')
            require(c['icon']['sprite']=='conviction','CONVICTION_ICON_SPRITE')
            require(c['icon']==icon('conviction',c['source_info_id']),'ICON_ASSET_BINDING')
            require([g['stage'] for g in c['augment_stages']]==([1,2] if c['category']=='augmentation' else []),'AUGMENT_STAGES')
            expected_unique=reference_parameters['unique_conviction'].get(c['key'])
            require((c['category']=='unique')==(c['unique_parameters'] is not None),'UNIQUE_PARAMETER_PRESENCE')
            require(c['category']!='unique' or expected_unique is not None,'UNKNOWN_UNIQUE_KEY')
            if expected_unique is not None:
                expected={e['kind']:e['unit'] for e in expected_unique['numeric_effects']}
                actual={e['kind']:e['unit'] for e in c['unique_parameters']['numeric_effects']}
                require(actual==expected and len(actual)==len(c['unique_parameters']['numeric_effects']),'UNIQUE_PARAMETERS')
                require(c['unique_parameters']['behaviors']==expected_unique['behaviors'] and c['unique_parameters']['targets']==expected_unique['targets'],'UNIQUE_BEHAVIOR_BINDING')
                if c['key']=='battle_instinct':
                    values={e['kind']:e['value'] for e in c['unique_parameters']['numeric_effects']}
                    threshold=values['adjacent_creature_threshold'];cap=values['adjacent_creature_cap']
                    require(threshold==int(threshold) and cap==int(cap) and 0<threshold<=cap<=8,'ADJACENT_CREATURE_BOUNDS')
            expected_units={'base_damage_bonus':'percent_points','base_healing_bonus':'percent_points','cooldown_reduction':'seconds','secondary_cooldown_reduction':'seconds','mana_cost_reduction':'mana','life_leech':'percent_points','critical_hit_chance':'percent_points','critical_extra_damage':'percent_points','additional_targets':'targets','range_increase':'tiles','next_attack_damage_reduction':'percent_points','duration_increase':'seconds'}
            for stage in c['augment_stages']:
                for effect in stage['numeric_effects']:require(effect['unit']==expected_units[effect['kind']],'AUGMENT_EFFECT_UNIT')
        require(set(r['domain'] for r in data['revelations'])==set(DOMAINS.values()),'REVELATION_DOMAINS')
        for rev in data['revelations']:
            require([(r['stage'],r['minimum_domain_points']) for r in rev['stages']]==[(1,250),(2,500),(3,1000)],'REVELATION_THRESHOLDS')
            require(rev['key'] in reference_parameters['revelations'],'UNKNOWN_REVELATION_KEY')
            require(rev['icon']==icon('revelation',rev['source_info_id']),'ICON_ASSET_BINDING')
            require(rev['behavior_rules']==reference_parameters['revelation_behaviors'][rev['key']],'REVELATION_BEHAVIORS')
            expected_kinds={e['kind'] for e in reference_parameters['revelations'][rev['key']]}
            for stage in rev['stages']:
                kinds=[e['kind'] for e in stage['numeric_effects']]
                require(len(kinds)==len(set(kinds)) and set(kinds)==expected_kinds,'REVELATION_PARAMETERS')
            require(rev['icon']['source_index']==rev['source_info_id'] and rev['icon']['sprite']=='revelation','REVELATION_ICON')
    gems=candidate['gems']
    for field in ['basic_mods','supreme_mods']:
        mods=gems[field];require(len({m['source_id'] for m in mods})==len(mods),'DUPLICATE_MOD_ID')
        for m in mods:
            require(m['icon']==icon('basic_mod' if field=='basic_mods' else 'supreme_mod',m['source_id']),'ICON_ASSET_BINDING')
            require(m['icon']['source_index']==m['source_id'] and m['icon']['sprite']==('basic_mod' if field=='basic_mods' else 'supreme_mod'),'MOD_ICON')
    basic_ids={m['source_id'] for m in gems['basic_mods']}
    require(not basic_ids.intersection(gems['excluded_empty_basic_mod_ids']),'EMPTY_MOD_ADMITTED')
    supreme_ids={m['source_id'] for m in gems['supreme_mods']}
    for v in candidate['vocations'].values():
        require(set(v['basic_mods_position_1'])<=basic_ids and set(v['basic_mods_position_2'])<=basic_ids and set(v['supreme_mods'])<=supreme_ids,'UNKNOWN_MOD_REFERENCE')
    for mod in gems['supreme_mods']:require([g['grade'] for g in mod['grades']]==[0,1,2,3],'MOD_GRADES')
    require([c['target_grade'] for c in gems['grade_costs']]==[1,2,3],'GRADE_COST_IDENTITIES')
    require([(q['quality'],q['basic_mod_count'],q['supreme_mod_count'],q['matching_vessel_stage'],q['matching_damage_healing_bonus']) for q in gems['qualities']]==[('lesser',1,0,1,1),('regular',2,0,2,1),('greater',2,1,3,2)],'GEM_QUALITY_SHAPE')
    for y in gems['atelier']['fragment_yields'].values():
        require(all(bounds[0]<=bounds[1] for bounds in [y['revealed'],y['unrevealed']]),'FRAGMENT_YIELD_RANGE')
    require(gems['atelier']['clockwise_domains']==['green','red','purple','blue'],'CLOCKWISE_DOMAINS')
    for mod in gems['supreme_mods']:
        if any(e['kind']=='cooldown_reduction' for g in mod['grades'] for e in g['numeric_effects']):
            reductions=[next(e['value'] for e in g['numeric_effects'] if e['kind']=='cooldown_reduction') for g in mod['grades']]
            require(len(set(reductions))==1,'COOLDOWN_GRADES_REQUIRE_MOMENTUM')
    loot=gems['loot_reference']
    require(loot['roll_denominator']>0,'LOOT_DENOMINATOR')
    for quality in loot['per_quality']:
        require(all(0<=chance<=loot['roll_denominator'] for chance in quality['chance_by_category'].values()),'LOOT_CHANCE_BOUNDS')
    # Reference identities, placement, complete effect shapes and policies must agree
    # with the hash-bound captures. Numeric effect tuning remains possible.
    reference=build()
    require(candidate['sources']==reference['sources'],'SOURCE_REVISION_BINDING')
    require(candidate['icon_evidence']==reference['icon_evidence'],'ICON_EVIDENCE_BINDING')
    require(candidate['verification']==reference['verification'],'VERIFICATION_CLAIM_BINDING')
    require(structure(candidate)==structure(reference),'REFERENCE_STRUCTURE_MISMATCH')

def validate_evidence(candidate, candidate_bytes, evidence_path=None):
    """Qualify an exact serialized candidate separately from semantic authoring."""
    evidence=read(evidence_path or ROOT/'samples/verification-evidence.json')
    Draft202012Validator(read(ROOT/'verification.schema.json')).validate(evidence)
    def require(condition, code):
        if not condition:raise ValueError(code)
    require(evidence['candidate_sha256']==hashlib.sha256(candidate_bytes).hexdigest(),'EVIDENCE_CANDIDATE_DIGEST')
    require(candidate==json.loads(candidate_bytes),'EVIDENCE_CANDIDATE_CONTENT')
    require(evidence['source_conflicts']==candidate['gems']['reference_corrections'],'EVIDENCE_CORRECTIONS')
    require(evidence['icon_evidence']==candidate['icon_evidence'],'EVIDENCE_ICONS')
    require(evidence['planner_replay']['source_commit']==candidate['sources'][0]['commit'],'EVIDENCE_SOURCE_REVISION')
    counts=evidence['counts']
    actual={'vocations':len(candidate['vocations']),
        'slots':sum(len(v['slots']) for v in candidate['vocations'].values()),
        'revelation_assignments':sum(len(v['revelations']) for v in candidate['vocations'].values()),
        'unique_conviction_types':len({s['conviction']['key'] for v in candidate['vocations'].values() for s in v['slots'] if s['conviction']['category']=='unique'}),
        'basic_mods':len(candidate['gems']['basic_mods']),'supreme_mods':len(candidate['gems']['supreme_mods']),
        'revelation_numeric_values':sum(len(s['numeric_effects']) for v in candidate['vocations'].values() for r in v['revelations'] for s in r['stages'])}
    require(counts==actual,'EVIDENCE_COUNTS')
    binding=evidence['live_source_audit'];require(binding['file']=='samples/live-source-audit.json','EVIDENCE_AUDIT_PATH')
    require(binding['sha256']==hashlib.sha256((ROOT/binding['file']).read_bytes()).hexdigest(),'EVIDENCE_AUDIT_DIGEST')
    audit=read(ROOT/binding['file'])
    item_binding=evidence['item_asset_reference']
    require(item_binding['file']=='samples/item-asset-reference.json','EVIDENCE_ITEM_ASSET_PATH')
    require(item_binding['sha256']==hashlib.sha256((ROOT/item_binding['file']).read_bytes()).hexdigest(),'EVIDENCE_ITEM_ASSET_DIGEST')
    from verify_item_assets import build_reference
    require(read(ROOT/item_binding['file'])==build_reference(),'EVIDENCE_ITEM_ASSET_INPUTS')
    require(not audit['runtime_admitted'] and not audit['live_global_parity_confirmed'] and not evidence['live_verification']['live_global_parity_confirmed'],'EVIDENCE_ADMISSION')
    if candidate['verification']['live_website_verified']:
        observations={o['key']:o for o in audit['http_observations']}
        for key in ('module','library'):
            observation=observations[key]
            require(observation['status']==200 and observation['equals_pinned_bytes'] and observation['sha256']==observation['pinned_sha256'],'EVIDENCE_LIVE_INPUTS')
        require(observations['renderer']['sha256']==evidence['icon_evidence']['renderer_sha256'],'EVIDENCE_RENDERER')
def main():
    parser=argparse.ArgumentParser();parser.add_argument('command',choices=['build','validate']);parser.add_argument('--check',action='store_true');parser.add_argument('--file',type=Path,default=ROOT/'samples/wheel-candidate.json');parser.add_argument('--previous',type=Path);parser.add_argument('--evidence',type=Path);args=parser.parse_args()
    qualified=False
    if args.command=='build':
        candidate=build();validate(candidate)
        if args.check:
            if args.file.read_text()!=json.dumps(candidate,indent=2,allow_nan=False)+'\n':raise ValueError('REBUILD_DRIFT')
        else:args.file.write_text(json.dumps(candidate,indent=2,allow_nan=False)+'\n')
    else:
        candidate=read(args.file);validate(candidate,read(args.previous) if args.previous else None)
    if args.evidence or ((args.command=='validate' or args.check) and args.file.resolve()==(ROOT/'samples/wheel-candidate.json').resolve()):
        validate_evidence(candidate,args.file.read_bytes(),args.evidence);qualified=True
    print('PASS: Wheel reference candidate; '+('exact-file evidence qualified.' if qualified else 'semantic validation only; evidence not qualified.'))
if __name__=='__main__':main()
