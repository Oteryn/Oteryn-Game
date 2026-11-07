"""Focused verification requested by the owner; fixtures are synthetic, not imported content."""
import copy
import argparse
import json
from decimal import localcontext
from tempfile import TemporaryDirectory
from importlib.metadata import version
from pathlib import Path
from jsonschema import Draft202012Validator
from normalize_monster_fields import cast_geometry, health_disposition
from validate_monster import validate,SCHEMAS,structural,ROOT,percent_to_ppm,read,resolve_pointer

def ident(name):return {'key':'oteryn:'+name,'revision':'r1'}
def ref(family,name):return {'family':family,**ident(name)}
def item(name,corpse=False,capacity=None,action='none',target=None):
    v={'identity':ident(name),'presentation':{'name':'Source item','asset_binding':'oteryn:body_sprite'},
       'classification':{'is_corpse':corpse},'physical':{'weight_centioz':60000,'movable':True,'pickupable':True},
       'collision':{'block_walk':False,'block_projectiles':False,'block_pathfinding':False},
       'temporal':{'decay_action':action,'stop_duration':False}}
    if capacity:v['container']={'capacity':capacity}
    if action!='none':v['temporal']['duration_ms']=10000
    if target:v['temporal']['decay_target']=ref('Item',target)
    return v

def fixture():
    monster={
      'creature':{'identity':ident('creature'),'display_name':'Source Creature','inspection':{'description':'a source creature'},
        'stats':{'max_health':260,'initial_health':260,'experience':150,'speed':95,'armor':17,'defense':20,'critical_chance_percent':0},
        'resistances':[{'damage_type':'energy','reduction_percent':{'numerator':25,'denominator':1}}],
        'immunities':{'damage_types':[],'conditions':[]},'flags':{'attackable':True,'illusionable':True,'health_hidden':False},
        'summoning':{'summonable':True,'mana_cost':490,'convinceable':False,'is_familiar':False},
        'bestiary':{'class':'Giant','taxonomy':'giant','difficulty':'medium','occurrence':'common','kill_thresholds':[25,250,500],'charm_points':15},
        'corpse_item':ref('Item','body'),'encyclopedia':{'description_document':ref('Document','lore')},
        'presentation':ref('Presentation','presentation'),'behavior':ref('Behavior','behavior'),'loot':ref('Loot','loot'),
        'damage_reflection':[],'healing_from_damage':[]},
      'behavior':{'identity':ident('behavior'),
        'movement':{'can_walk':True,'pass_through':False,'pushable':False,'push_items':True,'push_creatures':True,'field_permissions':{'energy':False,'fire':False,'poison':False}},
        'targeting':{'hostile':True,'can_target':True,'sense_invisible':False,'target_distance_tiles':1,'static_attack_chance_percent':90,
          'change_target':{'interval_ms':4000,'chance_percent':10},'strategy_weights':{'nearest':70,'damage':30,'health':0,'random':0},'flee_health':0},
        'attacks':[{'ability':ref('Ability','melee'),'interval_ms':2000,'chance_percent':100}],
        'defenses':[],'voices':{'interval_ms':5000,'chance_percent':10,'entries':[{'text':'Source spelling stays unchanged!','mode':'say'}]},'event_bindings':[]},
      'presentation':{'identity':ident('presentation'),'appearance':{'asset_binding':'oteryn:creature_sprite','palette_bindings':[],
        'attachment_bindings':[],'visual_effect_bindings':[]},'light':{'level':0},'audio':{'event_bindings':[]}},
      'loot':{'identity':ident('loot'),'algorithm':'IndependentBernoulli','entries':[{'item':ref('Item','coin'),'min_count':1,
        'max_count':47,'probability_percent':82,'skip_later_same_item_after_success':False}]}}
    deps={'abilities':[{'identity':ident('melee'),'kind':'melee','range_tiles':1,'needs_target':True,'needs_direction':False,'effects':[ref('Effect','hit')]}],
      'effects':[{'identity':ident('hit'),'operation':'damage','damage_type':'physical','formula':ref('Formula','damage')}],
      'formulas':[{'identity':ident('damage'),'kind':'range','magnitude':{'minimum':0,'maximum':105}}],
      'documents':[{'identity':ident('lore'),'document_type':'Note','title':'Original source text','language':'en',
        'content':['An original paragraph: punctuation, spelling and language remain unchanged.']}],
      'items':[item('body',True,30,'transform','old_body'),item('old_body',True,30,'remove'),item('coin')], 'loot_tables':[]}
    catalog={'definitions':[],'assets':['oteryn:creature_sprite','oteryn:body_sprite']}
    return monster,deps,catalog

results=[]
def case(name,mutate=None,expected=False):
    m,d,c=fixture()
    manifest=None
    if mutate:manifest=mutate(m,d,c)
    if not isinstance(manifest,dict) or 'sources' not in manifest:manifest=None
    errs=validate(m,d,c,manifest)
    ok=(not errs)==expected
    results.append({'name':name,'expected_valid':expected,'passed':ok,'error_count':len(errs),'first_error':errs[0] if errs else None})

def set_value(path,value):
    def mutate(m,d,c):
        cursor={'m':m,'d':d,'c':c}
        for part in path[:-1]:cursor=cursor[part]
        cursor[path[-1]]=value
    return mutate
def append_value(path,value):
    def mutate(m,d,c):
        cursor={'m':m,'d':d,'c':c}
        for part in path:cursor=cursor[part]
        cursor.append(value)
    return mutate

def nested(m,d,c):
    d['items'].append(item('bag',capacity=10))
    d['loot_tables'].append({'identity':ident('inside'),'algorithm':'IndependentBernoulli','entries':[]})
    m['loot']['entries'][0]['item']=ref('Item','bag');m['loot']['entries'][0]['contents_loot']=ref('Loot','inside')
def condition(m,d,c):
    d['effects'].append({'identity':ident('burn'),'operation':'condition','condition':{'type':'fire','lifetime':'damage_schedule','damage_over_time':{
        'total_damage_range':{'minimum':50,'maximum':50},'tick_interval_ms':1000,
        'initial_tick':{'mode':'automatic'},'tick_profile':'decreasing','first_tick':'after_interval'}}})
    d['abilities'][0]['effects'].append(ref('Effect','burn'))
def familiar(m,d,c):
    m['creature']['summoning']['is_familiar']=True
    m['creature']['summoning']['familiar']={'vocation':'knight','summon_ability':ref('Ability','melee'),'duration_ms':600000,'mana_cost':100}
def audio(m,d,c):
    m['presentation']['audio']['event_bindings']=[{'event':'periodic','cue_id':'grunt','asset_binding':'oteryn:grunt'}]
    m['behavior']['periodic_audio']={'interval_ms':5000,'chance_percent':10,'cue_ids':['grunt']}
    c['assets'].append('oteryn:grunt')
def manifest(status='mapped',kind='field',destination='/monster/creature/stats/max_health'):
    return {'sources':[{'repository':'opentibiabr/canary','revision':'47dfd51f45280a59a1d3e50ba7edd573d7234446'}],
      'entries':[{'source_index':0,'source_file':'data-otservbr-global/monster/giants/cyclops.lua','source_line':30,
        'source_field':'monster.maxHealth','kind':kind,'status':status,'destination':destination,'resolution':'Explicit source resolution for a synthetic validation case.'}]}

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output-dir',type=Path,default=ROOT)
    output=parser.parse_args().output_dir
    output.mkdir(parents=True,exist_ok=True)
    for name,s in SCHEMAS.items():
        Draft202012Validator.check_schema(s)
        results.append({'name':'meta-schema '+name,'passed':True})
    case('minimal complete synthetic fixture',expected=True)
    # CREATURE-AI-0 RL07/RL08: content validation, independent of runtime refusal.
    for field,maximum in [('attacks',16),('defenses',8),('summon_entries',8),('max_summons',16),('summon_count',16)]:
        def resource_limit(m,d,c,field=field,maximum=maximum,value=None):
            value=maximum if value is None else value
            if field in ('attacks','defenses'):
                m['behavior'][field]=[copy.deepcopy(m['behavior']['attacks'][0]) for _ in range(value)]
            else:
                entry={'creature':ref('Creature','creature'),'interval_ms':1000,'chance_percent':100,'count':16}
                m['behavior']['summons']={'max_summons':16,'entries':[entry]}
                if field=='summon_entries':m['behavior']['summons']['entries']=[copy.deepcopy(entry) for _ in range(value)]
                elif field=='max_summons':m['behavior']['summons']['max_summons']=value
                else:m['behavior']['summons']['entries'][0]['count']=value
        case('RL07/RL08 '+field+' exact maximum accepted',resource_limit,True)
        m,d,c=fixture();resource_limit(m,d,c,value=maximum+1)
        errors=structural('monster.schema.json',m)
        results.append({'name':'RL07/RL08 '+field+' first excess rejected structurally','passed':bool(errors),
                        'expected_valid':False,'error_count':len(errors),'first_error':errors[0] if errors else None})

    case('optional Bestiary notes accepted',set_value(['m','creature','bestiary','notes'],'A short Oteryn-authored account.'),True)
    case('empty Bestiary notes rejected',set_value(['m','creature','bestiary','notes'],''))
    case('condition with ticks',condition,True)
    case('speed formula closure',lambda m,d,c:(
        d['formulas'].append({'identity':ident('speed'),'kind':'speed_modifier','speed':{'minimum_multiplier':{'numerator':1,'denominator':4},
            'maximum_multiplier':{'numerator':1,'denominator':2},'minimum_offset':40,'maximum_offset':40}}),
        d['effects'].append({'identity':ident('slow'),'operation':'condition','duration_ms':10000,
            'condition':{'type':'paralyze','lifetime':'fixed_duration','speed_formula':ref('Formula','speed')}})),True)
    def paralysis_formula(formula):
        def mutate(m,d,c):
            d['formulas'].append({'identity':ident('paralysis'),**formula})
            d['effects'].append({'identity':ident('slow'),'operation':'condition','duration_ms':10000,
                'condition':{'type':'paralyze','lifetime':'fixed_duration','speed_formula':ref('Formula','paralysis')}})
        return mutate
    for kind,payload in [('range',{'magnitude':{'minimum':0,'maximum':100}}),
                         ('melee_attack_skill',{'melee':{'attack':10,'skill':10}}),('caster_magnitude',{})]:
        case('paralysis rejects '+kind+' Formula',paralysis_formula({'kind':kind,**payload}))
    case('speed formula mutually exclusive branches',lambda m,d,c:d['formulas'][0].update(kind='speed_modifier',speed={
        'minimum_multiplier':{'numerator':1,'denominator':2},'maximum_multiplier':{'numerator':1,'denominator':1},'minimum_offset':40,'maximum_offset':40}))
    case('familiar with required profile',familiar,True)
    case('nested loot selecting a container',nested,True)
    case('periodic audio resolves',audio,True)
    case('100 percent attack',set_value(['m','behavior','attacks',0,'chance_percent'],100),True)
    case('zero probability loot',set_value(['m','loot','entries',0,'probability_percent'],0),True)
    case('below-zero probability',set_value(['m','loot','entries',0,'probability_percent'],-1))
    case('above-100 percent probability',set_value(['m','behavior','attacks',0,'chance_percent'],100.0001))
    case('fractional percentage accepted',set_value(['m','behavior','voices','chance_percent'],0.19),True)
    case('one ppm precision accepted as percent',set_value(['m','behavior','voices','chance_percent'],0.0001),True)
    case('excess precision rejected',set_value(['m','behavior','voices','chance_percent'],0.00001))
    case('ppm field rejected in authoring',set_value(['m','behavior','voices','chance_ppm'],1900))
    case('old unscaled chance rejected',set_value(['m','behavior','attacks',0,'chance'],10))
    case('required value null',set_value(['m','creature','stats','armor'],None))
    case('unknown monster field',set_value(['m','creature','unexpected'],True))
    case('max and initial HP ordering',set_value(['m','creature','stats','initial_health'],261))
    case('flee threshold exceeds HP',set_value(['m','behavior','targeting','flee_health'],300))
    case('missing summon cost',lambda m,d,c:m['creature']['summoning'].pop('mana_cost'))
    case('shared cost forbidden when neither flag applies',lambda m,d,c:m['creature']['summoning'].update(summonable=False))
    case('familiar flag without profile',set_value(['m','creature','summoning','is_familiar'],True))
    case('unordered Bestiary thresholds',set_value(['m','creature','bestiary','kill_thresholds'],[250,25,500]))
    case('duplicate Bestiary thresholds',set_value(['m','creature','bestiary','kill_thresholds'],[25,25,500]))
    case('non-canonical ratio rejected',set_value(['m','creature','resistances',0,'reduction_percent'],{'numerator':50,'denominator':2}))
    case('non-canonical zero ratio rejected',set_value(['m','creature','resistances',0,'reduction_percent'],{'numerator':0,'denominator':5}))
    case('canonical zero ratio accepted',set_value(['m','creature','resistances',0,'reduction_percent'],{'numerator':0,'denominator':1}),True)
    case('non-canonical speed multiplier rejected',lambda m,d,c:(
        d['formulas'].append({'identity':ident('speed'),'kind':'speed_modifier','speed':{'minimum_multiplier':{'numerator':2,'denominator':8},
            'maximum_multiplier':{'numerator':1,'denominator':2},'minimum_offset':40,'maximum_offset':40}}),
        d['effects'].append({'identity':ident('slow'),'operation':'condition','duration_ms':10000,
            'condition':{'type':'paralyze','lifetime':'fixed_duration','speed_formula':ref('Formula','speed')}})))
    case('excess precision rejected on loot probability',set_value(['m','loot','entries',0,'probability_percent'],82.00001))
    case('excess precision rejected on critical chance',set_value(['m','creature','stats','critical_chance_percent'],0.12345))
    case('death residue with fluid accepted',set_value(['m','creature','death_residue'],{'item':ref('Item','coin'),'fluid_type':'blood'}),True)
    case('death residue must resolve its Item',set_value(['m','creature','death_residue'],{'item':ref('Item','missing'),'fluid_type':'blood'}))
    case('obsolete death_residue_item rejected',set_value(['m','creature','death_residue_item'],ref('Item','coin')))
    boss={'category':'nemesis','prowess_kills':1,'expertise_kills':3,'mastery_kills':5,'prowess_points':10,'expertise_points':30,'mastery_points':60}
    case('per-stage bosstiary points accepted',set_value(['m','creature','bosstiary'],boss),True)
    case('obsolete single boss_points rejected',set_value(['m','creature','bosstiary'],{**boss,'boss_points':10}))
    case('bosstiary stage points must increase',set_value(['m','creature','bosstiary'],{**boss,'expertise_points':5}))
    case('zero denominator',set_value(['m','creature','resistances',0,'reduction_percent','denominator'],0))
    case('resistance over 100 percent',set_value(['m','creature','resistances',0,'reduction_percent','numerator'],101))
    case('negative resistance supports vulnerability',set_value(['m','creature','resistances',0,'reduction_percent','numerator'],-10),True)
    case('duplicate element',append_value(['m','creature','resistances'],{'damage_type':'energy','reduction_percent':{'numerator':10,'denominator':1}}))
    case('all-zero targeting weights',set_value(['m','behavior','targeting','strategy_weights'],{'nearest':0,'damage':0,'health':0,'random':0}))
    case('missing exact ability revision',set_value(['m','behavior','attacks',0,'ability','revision'],'r2'))
    case('wrong reference family',set_value(['m','behavior','attacks',0,'ability','family'],'Item'))
    case('missing asset binding',set_value(['m','presentation','appearance','asset_binding'],'oteryn:missing'))
    case('malformed external family rejected',append_value(['c','definitions'],{'family':[],'key':'oteryn:external','revision':'r1'}))
    case('invalid asset catalog key rejected',append_value(['c','assets'],'unqualified'))
    case('bad palette binding shape',set_value(['m','presentation','appearance','palette_bindings'],[{}]))
    case('old color number rejected',set_value(['m','presentation','light','color'],215))
    case('nonzero light without color',set_value(['m','presentation','light','level'],1))
    case('min/max loot ordering',set_value(['m','loot','entries',0,'min_count'],48))
    case('min/max damage ordering',set_value(['d','formulas',0,'magnitude','minimum'],106))
    case('melee and range formulas mutually exclusive',set_value(['d','formulas',0,'melee'],{'attack':10,'skill':10}))
    case('condition damage requires tick interval',lambda m,d,c:(condition(m,d,c),d['effects'][-1]['condition']['damage_over_time'].pop('tick_interval_ms')))
    case('create item operation resolves',lambda m,d,c:d['effects'].append({'identity':ident('field'),'operation':'create_item','created_item':ref('Item','coin')}),True)
    case('transform needs duration and target',lambda m,d,c:d['effects'].append({'identity':ident('outfit'),'operation':'appearance_transform'}))
    def spell_area(area):
        def mutate(m,d,c):
            d['abilities'][0].update(kind='spell',area=area)
        return mutate
    case('area matrix accepted (D12)',spell_area({'matrix':{'north':['.x.','xCx','.x.'],'diagonal':['xx.','xC.','...']}}),True)
    case('area matrix needs exactly one centre',spell_area({'matrix':{'north':['xCx','.C.']}}))
    case('area matrix rows must be rectangular',spell_area({'matrix':{'north':['xCx','x']}}))
    case('area matrix excludes radius',spell_area({'matrix':{'north':['C']},'radius_tiles':2}))
    case('area matrix cell alphabet',spell_area({'matrix':{'north':['1C1']}}))
    def dot(damage_over_time):
        def mutate(m,d,c):
            d['effects'].append({'identity':ident('dot'),'operation':'condition',
                'condition':{'type':'energy','lifetime':'damage_schedule','damage_over_time':damage_over_time}})
            d['abilities'][0]['effects'].append(ref('Effect','dot'))
        return mutate
    fixed={'tick_profile':'fixed','first_tick':'after_interval','fixed_ticks':[{'count':8,'interval_ms':10000,'amount':25}]}
    case('fixed-tick DoT accepted (D12)',dot(fixed),True)
    case('fixed-tick DoT forbids total range',dot({**fixed,'total_damage_range':{'minimum':1,'maximum':2}}))
    case('decreasing DoT forbids fixed ticks',dot({'tick_profile':'decreasing','first_tick':'immediate','fixed_ticks':fixed['fixed_ticks'],
        'total_damage_range':{'minimum':1,'maximum':2},'tick_interval_ms':2000,'initial_tick':{'mode':'automatic'}}))
    geometric={'tick_profile':'geometric','first_tick':'after_interval','geometric':{'base_range':{'minimum':40,'maximum':170},
        'factor':{'numerator':6,'denominator':5},'tick_counts':[5,6,7],'tick_interval_ms':4000}}
    case('geometric DoT accepted (D21)',dot(geometric),True)
    def geometric_value(field,value):
        changed=copy.deepcopy(geometric)
        changed['geometric'][field]=value
        return dot(changed)
    case('geometric DoT rejects reversed base range',geometric_value('base_range',{'minimum':171,'maximum':170}))
    case('geometric DoT accepts equal base bounds',geometric_value('base_range',{'minimum':40,'maximum':40}),True)
    case('geometric DoT rejects negative factor',geometric_value('factor',{'numerator':-6,'denominator':5}))
    case('geometric DoT accepts constant factor',geometric_value('factor',{'numerator':1,'denominator':1}),True)
    case('geometric DoT accepts nonnegative decreasing factor',geometric_value('factor',{'numerator':1,'denominator':2}),True)
    case('geometric DoT forbids a total range',dot({**geometric,'total_damage_range':{'minimum':1,'maximum':2}}))
    case('geometric DoT needs tick counts',dot({**geometric,'geometric':{k:v for k,v in geometric['geometric'].items() if k!='tick_counts'}}))
    case('decreasing DoT forbids geometric ticks',dot({'tick_profile':'decreasing','first_tick':'immediate','geometric':geometric['geometric'],
        'total_damage_range':{'minimum':1,'maximum':2},'tick_interval_ms':2000,'initial_tick':{'mode':'automatic'}}))
    def attributes(modifiers):
        def mutate(m,d,c):
            d['effects'].append({'identity':ident('weak'),'operation':'condition','duration_ms':8000,
                'condition':{'type':'attributes','lifetime':'fixed_duration','attribute_modifiers':modifiers}})
            d['abilities'][0]['effects'].append(ref('Effect','weak'))
        return mutate
    case('attribute modifiers accepted (D12)',attributes([{'attribute':'skill_shield','mode':'percent_of_base','value':40}]),True)
    case('absolute attribute modifier accepted',attributes([{'attribute':'skill_shield','mode':'add','value':-10}]),True)
    case('attributes condition rejects missing modifiers',lambda m,d,c:(
        attributes([{'attribute':'skill_shield','mode':'add','value':-10}])(m,d,c),
        d['effects'][-1]['condition'].pop('attribute_modifiers')))
    case('attributes condition rejects empty modifiers',attributes([]))
    case('attribute modifier mode is closed',attributes([{'attribute':'skill_shield','mode':'multiply','value':40}]))
    case('damage mitigated by armor and shield accepted',set_value(('d','effects',0,'mitigated_by'),['armor','shield']),True)
    case('mitigation defence is closed',set_value(('d','effects',0,'mitigated_by'),['armor','magic_shield']))
    case('mitigation defences are unique',set_value(('d','effects',0,'mitigated_by'),['armor','armor']))
    case('mitigation list is not empty',set_value(('d','effects',0,'mitigated_by'),[]))
    def healed(m,d,c):
        d['effects'][0].update({'operation':'heal','damage_type':'healing','mitigated_by':['armor']})
    case('heal cannot be mitigated by armor',healed)
    def dispel(body):
        def mutate(m,d,c):
            d['effects'].append({'identity':ident('dispel'),'operation':'remove_condition',**body})
            d['abilities'][0]['effects'].append(ref('Effect','dispel'))
        return mutate
    case('remove condition accepted (DISPEL)',dispel({'removed_condition':'paralyze'}),True)
    case('remove condition names its condition',dispel({}))
    case('remove condition has no duration',dispel({'removed_condition':'paralyze','duration_ms':1000}))
    case('damage cannot remove a condition',set_value(('d','effects',0,'removed_condition'),'paralyze'))
    def extra_effect(body):
        def mutate(m,d,c):
            d['effects'].append({'identity':ident('extra'),**body})
            d['abilities'][0]['effects'].append(ref('Effect','extra'))
            c['definitions'].append(ref('Creature','minion'))
        return mutate
    summon={'creatures':[ref('Creature','minion')],'count_mode':'fill_to_limit','count':4,'only_below_summons':4,'owned':True,'max_offset_tiles':0}
    case('summon creature accepted (D18)',extra_effect({'operation':'summon_creature','summon':summon}),True)
    def undeclared(m,d,c):
        d['effects'].append({'identity':ident('extra'),'operation':'summon_creature','summon':{**summon,'creatures':[ref('Creature','ghost')]}})
        d['abilities'][0]['effects'].append(ref('Effect','extra'))
    case('summoned creature must be declared',undeclared)
    case('summon count mode is closed',extra_effect({'operation':'summon_creature','summon':{**summon,'count_mode':'random'}}))
    case('summon needs a creature',extra_effect({'operation':'summon_creature','summon':{**summon,'creatures':[]}}))
    case('summon requires its parameters',extra_effect({'operation':'summon_creature'}))
    case('only summon_creature carries summon',extra_effect({'operation':'presentation_only','presentation':{'impact_asset_binding':'oteryn:body_sprite'},'summon':summon}))
    removal={'items':[ref('Item','coin')],'selection':'first_listed_per_tile'}
    case('remove items accepted (D18)',extra_effect({'operation':'remove_items','removed_items':removal}),True)
    case('remove items top item of the first tile accepted (SW-2)',extra_effect({'operation':'remove_items','removed_items':{**removal,'selection':'top_item_first_tile'}}),True)
    case('remove items needs an item',extra_effect({'operation':'remove_items','removed_items':{**removal,'items':[],'selection':'top_item_first_tile'}}))
    case('remove items selection is closed',extra_effect({'operation':'remove_items','removed_items':{**removal,'selection':'all'}}))
    affects={'kind':'masterless_monsters','top_creature_only':False,'excludes_caster_name':False,'includes_caster':True}
    case('heal affects allies accepted (D18)',set_value(('d','effects',0,'affects'),affects),True)
    case('named affects need creatures',set_value(('d','effects',0,'affects'),{**affects,'kind':'named_creatures'}))
    case('named affects accepted',set_value(('d','effects',0,'affects'),{**affects,'kind':'named_creatures','creatures':[ref('Creature','creature')]}),True)
    case('group affects forbid creatures',set_value(('d','effects',0,'affects'),{**affects,'creatures':[ref('Creature','creature')]}))
    windup={'delay_ms':2000,'caster_asset_binding':'oteryn:body_sprite'}
    def targeted(extra):
        def mutate(m,d,c):
            d['abilities'][0].update({'needs_target':True,'windup':windup,**extra})
        return mutate
    case('windup accepted on a single-target ability (SW-1)',targeted({}),True)
    case('windup delay must be positive',targeted({'windup':{**windup,'delay_ms':0}}))
    case('windup needs a caster binding',targeted({'windup':{'delay_ms':2000}}))
    case('windup needs a target',targeted({'needs_target':False}))
    case('windup rejected with an area',targeted({'area':{'radius_tiles':1}}))
    case('path requirement accepted (D18)',set_value(('d','abilities',0,'path_requirement'),{'max_search_tiles':8,'clear_sight':True}),True)
    case('path requirement needs its search distance',set_value(('d','abilities',0,'path_requirement'),{'clear_sight':True}))
    case('path trail presentation accepted',set_value(('d','effects',0,'presentation'),{'path_asset_binding':'oteryn:body_sprite'}),True)
    def invisible(extra=None):
        def mutate(m,d,c):
            appearance=m['presentation']['appearance']
            appearance.pop('asset_binding'); appearance['selection']='invisible'
            if extra: appearance.update(extra)
        return mutate
    case('invisible appearance accepted (D24)',invisible(),True)
    case('invisible appearance forbids an asset',invisible({'asset_binding':'oteryn:creature_sprite'}))
    case('visible appearance needs an asset',lambda m,d,c:m['presentation']['appearance'].pop('asset_binding'))
    case('affects kind is closed',set_value(('d','effects',0,'affects'),{**affects,'kind':'everyone'}))
    def variants(nested=False,with_effects=False):
        def mutate(m,d,c):
            base=d['abilities'][0]
            for n in (1,2):
                variant={**base,'identity':ident(f'melee-{n}')}
                if nested:variant={k:v for k,v in variant.items() if k!='effects'}|{'variants':[ref('Ability','melee'),ref('Ability','melee')]}
                d['abilities'].append(variant)
            d['abilities'][0]={k:v for k,v in base.items() if with_effects or k!='effects'}|{'variants':[ref('Ability','melee-1'),ref('Ability','melee-2')]}
        return mutate
    case('ability variants accepted (D12)',variants(),True)
    case('ability variants exclude effects',variants(with_effects=True))
    case('ability variant cannot nest variants',variants(nested=True))
    def encounter_cast(with_effects=False,kind='spell'):
        def mutate(m,d,c):
            base=d['abilities'][0]
            encounter=ref('Encounter','canary:encounter/boss')
            d['abilities'][0]={k:v for k,v in base.items() if with_effects or k!='effects'}|{'encounter':encounter,'kind':kind}
            c['definitions'].append(encounter)
        return mutate
    case('encounter-driven spell accepted (D45)',encounter_cast(),True)
    case('encounter-driven ability excludes effects',encounter_cast(with_effects=True))
    case('encounter-driven ability must be a spell (D45)',encounter_cast(kind='melee'))
    def caster(magnitude):
        def mutate(m,d,c):
            d['formulas'][0]={'identity':d['formulas'][0]['identity'],'kind':'caster_magnitude'}
            if magnitude:m['behavior']['attacks'][0]['magnitude']=magnitude
        return mutate
    case('caster magnitude with schedule magnitude (D11)',caster({'minimum':100,'maximum':210}),True)
    case('caster magnitude requires schedule magnitude',caster(None))
    case('schedule magnitude minimum must not exceed maximum',caster({'minimum':300,'maximum':210}))
    case('chain targeting accepted (D12)',lambda m,d,c:d['abilities'][0].update(kind='spell',chain={'max_targets':2,'range_tiles':3,'backtracking':False}),True)
    case('players-only chain accepted',lambda m,d,c:d['abilities'][0].update(kind='spell',chain={'max_targets':2,'range_tiles':3,'backtracking':False,'target_filter':'players'}),True)
    case('spell chain fields accepted',lambda m,d,c:d['abilities'][0].update(kind='spell',chain={'max_targets':4,'range_tiles':4,'backtracking':False,'shape':'sequential','initial_range_tiles':4,'damage_step_percent':-5}),True)
    case('fork chain with ranged monster filter accepted',lambda m,d,c:d['abilities'][0].update(kind='spell',chain={'max_targets':5,'range_tiles':4,'backtracking':False,'shape':'fork','target_filter':'ranged_monsters'}),True)
    case('chain shape is closed',lambda m,d,c:d['abilities'][0].update(kind='spell',chain={'max_targets':2,'range_tiles':3,'backtracking':False,'shape':'star'}))
    case('chain initial range must be positive',lambda m,d,c:d['abilities'][0].update(kind='spell',chain={'max_targets':2,'range_tiles':3,'backtracking':False,'initial_range_tiles':0}))
    case('chain damage step is bounded',lambda m,d,c:d['abilities'][0].update(kind='spell',chain={'max_targets':2,'range_tiles':3,'backtracking':False,'damage_step_percent':-101}))
    case('chain target filter is closed',lambda m,d,c:d['abilities'][0].update(kind='spell',chain={'max_targets':2,'range_tiles':3,'backtracking':False,'target_filter':'monsters'}))
    case('any local Item may be the corpse (D17)',set_value(['d','items',0,'classification','is_corpse'],False),True)
    case('change_target interval 0 disables timed changes',
        lambda m,d,c:m['behavior']['targeting'].__setitem__('change_target',{'interval_ms':0,'chance_percent':8}),True)
    case('familiar look selection accepted (D16)',lambda m,d,c:m['presentation']['appearance'].__setitem__('selection','owner_familiar_look'),True)
    case('unknown appearance selection rejected',lambda m,d,c:m['presentation']['appearance'].__setitem__('selection','random'))
    case('decay transform needs target',lambda m,d,c:d['items'][0]['temporal'].pop('decay_target'))
    case('remove decay forbids target',set_value(['d','items',1,'temporal','decay_target'],ref('Item','coin')))
    case('decay cycle rejected',lambda m,d,c:d['items'][1]['temporal'].update(decay_action='transform',decay_target=ref('Item','body')))
    case('duplicate identity rejected',append_value(['d','items'],item('coin')))
    case('nested loot cycle rejected',lambda m,d,c:(nested(m,d,c),d['loot_tables'][0]['entries'].append({'item':ref('Item','bag'),'min_count':1,'max_count':1,'probability_percent':100,'skip_later_same_item_after_success':False,'contents_loot':ref('Loot','loot')})))
    case('child loot on non-container rejected',lambda m,d,c:(nested(m,d,c),m['loot']['entries'][0].update(item=ref('Item','coin'))))
    case('exclusive prey requires prey',set_value(['m','creature','system_eligibility'],{'prey':False,'exclusive_prey':True,'forge':False,'reward_boss':False}))
    case('mapped source manifest',lambda m,d,c:manifest(),True)
    case('unresolved source blocks readiness',lambda m,d,c:manifest('unresolved_semantics'))
    case('partial original prose blocks readiness',lambda m,d,c:manifest('partial_text','original_text'))
    case('script cannot be hidden as metadata',lambda m,d,c:manifest('metadata_only','script'))
    case('manifest missing destination',lambda m,d,c:manifest(destination='/monster/creature/missing'))
    wiki={'kind':'mediawiki','api':'https://tibia.fandom.com/api.php','title':'Synthetic page','page_id':1,'revision_id':1,
          'content_sha256':'0'*64}
    case('mediawiki revision source accepted',lambda m,d,c:{**manifest(),'sources':manifest()['sources']+[wiki]},True)
    case('mediawiki source requires content hash',
        lambda m,d,c:{**manifest(),'sources':manifest()['sources']+[{k:v for k,v in wiki.items() if k!='content_sha256'}]})
    case('mediawiki source rejects unpinned revision',lambda m,d,c:{**manifest(),'sources':manifest()['sources']+[{**wiki,'revision_id':0}]})
    case('git source rejects abbreviated revision',
        lambda m,d,c:{**manifest(),'sources':[{'repository':'opentibiabr/canary','revision':'47dfd51f'}]})
    case('manifest source index valid',lambda m,d,c:{**manifest(),'entries':[{**manifest()['entries'][0],'source_index':9}]} )

    for percent in (0.0003,0.29,1.4,4.93):
        case('exact decimal percent '+str(percent),set_value(['m','loot','entries',0,'probability_percent'],percent),True)
    case('zero minimum loot quantity retained',set_value(['m','loot','entries',0,'min_count'],0),True)
    case('negative minimum quantity rejected',set_value(['m','loot','entries',0,'min_count'],-1))
    case('zero maximum quantity rejected',set_value(['m','loot','entries',0,'max_count'],0))
    case('zero target distance retained',set_value(['m','behavior','targeting','target_distance_tiles'],0),True)
    case('negative target distance rejected',set_value(['m','behavior','targeting','target_distance_tiles'],-1))
    case('zero HP helper blocked',lambda m,d,c:m['creature']['stats'].update(initial_health=0,max_health=0))
    case('pass-through requires explicit value',lambda m,d,c:m['behavior']['movement'].pop('pass_through'))
    case('pass-through boolean accepted',set_value(['m','behavior','movement','pass_through'],True),True)
    case('pass-through cannot be a number',set_value(['m','behavior','movement','pass_through'],1))
    case('underground period override accepted',set_value(['m','creature','spawn_eligibility'],{
        'period':'day','ignore_period_underground':True,'blocked_by_nearby_players':True}),True)
    case('obsolete underground permission rejected',set_value(['m','creature','spawn_eligibility'],{
        'period':'day','allowed_underground':True,'blocked_by_nearby_players':True}))
    case('shared summon and convince mana cost',set_value(['m','creature','summoning','convinceable'],True),True)
    case('convince-only shared cost',lambda m,d,c:m['creature']['summoning'].update(summonable=False,convinceable=True),True)
    case('obsolete separate summon cost rejected',set_value(['m','creature','summoning','summon_mana_cost'],490))
    case('original Bestiary location and stars',lambda m,d,c:m['creature']['bestiary'].update(stars=2,locations='Source location text.'),True)
    case('Bestiary stars outside profile rejected',set_value(['m','creature','bestiary','stars'],6))
    def demon_infobox_fields(m,d,c):
        # Synthetic schema compatibility case for the owner's screenshot; not imported lore or mechanics.
        m['creature']['display_name']='Demon'
        m['creature']['stats'].update(experience=6000,max_health=8200,initial_health=8200,armor=44,speed=128,
                                     mitigation_percent={'numerator':69,'denominator':25})
        m['creature']['summoning'].update(summonable=False,convinceable=False)
        m['creature']['summoning'].pop('mana_cost')
        m['creature']['bestiary'].update(difficulty='hard',occurrence='common',stars=4,charm_points=50)
    case('Demon infobox fields represented exactly',demon_infobox_fields,True)
    case('direction required',lambda m,d,c:d['abilities'][0].pop('needs_direction'))
    case('mixed source geometry canonical circle with direction',lambda m,d,c:d['abilities'][0].update(
        cast_geometry(length=5,radius=2,target=False)),True)
    case('fixed-duration condition',lambda m,d,c:d['effects'].append({'identity':ident('fixed'),
        'operation':'condition','duration_ms':4000,'condition':{'type':'paralyze','lifetime':'fixed_duration'}}),True)
    case('fixed-duration condition requires duration',lambda m,d,c:d['effects'].append({'identity':ident('fixed'),
        'operation':'condition','condition':{'type':'paralyze','lifetime':'fixed_duration'}}))
    case('damage schedule forbids fabricated duration',lambda m,d,c:(condition(m,d,c),d['effects'][-1].update(duration_ms=5000)))
    case('damage schedule range ordering',lambda m,d,c:(condition(m,d,c),
        d['effects'][-1]['condition']['damage_over_time']['total_damage_range'].update(minimum=51)))
    case('fixed first damage tick',lambda m,d,c:(condition(m,d,c),
        d['effects'][-1]['condition']['damage_over_time'].update(initial_tick={'mode':'fixed','amount':5})),True)
    case('fixed first tick requires magnitude',lambda m,d,c:(condition(m,d,c),
        d['effects'][-1]['condition']['damage_over_time'].update(initial_tick={'mode':'fixed'})))
    case('automatic first tick forbids magnitude',lambda m,d,c:(condition(m,d,c),
        d['effects'][-1]['condition']['damage_over_time'].update(initial_tick={'mode':'automatic','amount':5})))
    case('obsolete damage-per-tick fields rejected',lambda m,d,c:(condition(m,d,c),
        d['effects'][-1]['condition'].update(damage_per_tick={'minimum':1,'maximum':2,'initial':1})))
    case('pure visual effect accepted',lambda m,d,c:d['effects'].append({'identity':ident('visual'),
        'operation':'presentation_only','presentation':{'impact_asset_binding':'oteryn:body_sprite'}}),True)
    case('pure visual effect requires a binding',lambda m,d,c:d['effects'].append({'identity':ident('visual'),
        'operation':'presentation_only','presentation':{}}))
    case('pure visual cannot carry damage',lambda m,d,c:d['effects'].append({'identity':ident('visual'),
        'operation':'presentation_only','presentation':{'impact_asset_binding':'oteryn:body_sprite'},
        'formula':ref('Formula','damage'),'damage_type':'physical'}))
    results.append({'name':'source radius replacement retains direction',
        'passed':cast_geometry(length=5,radius=2,target=False)=={'needs_target':False,'needs_direction':True,'area':{'radius_tiles':2}}})
    results.append({'name':'source nonpositive radius does not replace length',
        'passed':cast_geometry(length=5,spread=3,radius=0)['area']=={'length_tiles':5,'spread_tiles':3}})
    results.append({'name':'HP-zero helper requires explicit disposition',
        'passed':health_disposition(health=0,max_health=0)=='unresolved_semantics'})

    # Generic binary-float JSON Schema validation must accept valid percents; precision is a semantic rule.
    float_percent=Draft202012Validator(json.loads(json.dumps(SCHEMAS['monster.schema.json']['$defs']['percent'],default=float)))
    for literal,expected in [('0.0003',True),('0.29',True),('1.4',True),('4.93',True),('100',True),('-0.0001',False),('100.0001',False)]:
        results.append({'name':'binary-float structural percent '+literal,'passed':float_percent.is_valid(json.loads(literal))==expected})
    for percent,expected_ppm in [('0',0),('100',1000000),('0.19',1900),('0.0001',1),('82',820000)]:
        results.append({'name':'exact percent to native ppm '+percent,'passed':percent_to_ppm(percent)==expected_ppm})
    # Exercise the file reader, not a float approximation of the JSON literal.
    for precision in (6,28):
        with localcontext() as context, TemporaryDirectory() as directory:
            context.prec=precision
            for literal,expected_ppm in [
                ('0.29000000000000000000000000000001',None),
                ('99.999999999999999999999999999999',None),
                ('0.29000000001',None),
                ('2.9000000000000000000000000000001e-1',None),
                ('1e-100',None),
                ('0.29000000000000000000000000000000',2900),
                ('2.9000000000000000000000000000000e-1',2900),
                ('99.9999',999999),
                ('1e-4',1),
                ('100.000000000000000000000000000000',1000000),
                ('-0.000000000000000000000000000000',0),
            ]:
                m,d,c=fixture()
                m['loot']['entries'][0]['probability_percent']='EXACT_PERCENT_LITERAL'
                path=Path(directory)/'monster.json'
                path.write_text(json.dumps(m).replace('"EXACT_PERCENT_LITERAL"',literal),encoding='utf-8')
                parsed=read(path)
                errors=validate(parsed,d,c)
                try:
                    ppm=percent_to_ppm(parsed['loot']['entries'][0]['probability_percent'])
                    converted=True
                except ValueError:
                    ppm=None;converted=False
                valid=expected_ppm is not None
                results.append({'name':f'exact JSON percent precision={precision}: {literal}',
                    'passed':(not errors)==valid and converted==valid and ppm==expected_ppm,
                    'expected_valid':valid,'error_count':len(errors)})

    # Array syntax must not inherit Python's signed, padded, or Unicode indices.
    for token in ('-1','-0','00','01','+0',' 0','0 ','0_0','\u0660','\uff10','-',''):
        case('manifest rejects noncanonical array index '+repr(token),
            lambda m,d,c,token=token:manifest(destination='/monster/behavior/attacks/'+token+'/ability'))
    case('manifest canonical array index resolves',
        lambda m,d,c:manifest(destination='/monster/behavior/attacks/0/ability'),True)
    case('manifest out-of-bounds array index rejected',
        lambda m,d,c:manifest(destination='/monster/behavior/attacks/1/ability'))
    case('manifest rejects malformed pointer escape',
        lambda m,d,c:manifest(destination='/monster/creature/display_name~2'))

    pointer_document={'array':list(range(11)),'-1':'negative key','00':'padded key','+0':'signed key',
        'a/b':'slash key','m~n':'tilde key','~1':'literal escape','':'empty key',
        'bad~2':'not an escape','bad~':'trailing tilde'}
    for path,expected in [('',pointer_document),('/array/0',0),('/array/10',10),
        ('/-1','negative key'),('/00','padded key'),('/+0','signed key'),
        ('/a~1b','slash key'),('/m~0n','tilde key'),('/~01','literal escape'),('/','empty key')]:
        try:
            matched=resolve_pointer(pointer_document,path)==expected
        except (KeyError,IndexError,ValueError,TypeError):
            matched=False
        results.append({'name':'JSON pointer valid object/array token '+repr(path),'passed':matched})
    for path in ('/bad~2','/bad~','/array/0/x'):
        try:
            resolve_pointer(pointer_document,path)
            rejected=False
        except (KeyError,IndexError,ValueError,TypeError):
            rejected=True
        results.append({'name':'JSON pointer malformed escape/scalar traversal '+path,'passed':rejected})

    for name,schema_name in [('monster-template.json','monster.schema.json'),('monster-dependencies-template.json','monster-dependencies.schema.json')]:
        count=len(structural(schema_name,json.loads((ROOT/name).read_text(encoding='utf-8'))))
        results.append({'name':'empty placeholders are not ready data: '+name,'passed':count>0,'structural_errors':count})
    m,d,c=fixture()
    for name,value in [('synthetic-valid-monster.json',m),('synthetic-valid-dependencies.json',d),('synthetic-catalog.json',c)]:
        (output/name).write_text(json.dumps(value,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
    report={'scope':'local authoring schemas and semantic validator, synthetic fixtures only; no Lua or Oteryn runtime executed',
      'jsonschema_version':version('jsonschema'),'checks':len(results),'passed':sum(x['passed'] for x in results),
      'failed':sum(not x['passed'] for x in results),'results':results}
    (output/'formal-schema-validation-report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
    print(json.dumps({k:v for k,v in report.items() if k!='results'},ensure_ascii=False))
    for r in results:
        if not r['passed']:print(json.dumps(r,ensure_ascii=False))
    raise SystemExit(report['failed']>0)
