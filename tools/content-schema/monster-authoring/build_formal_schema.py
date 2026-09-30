"""Build the Monster authoring schema candidate v1. This is not WorldProject/v2 serialization."""
import copy
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent
DIALECT = 'https://json-schema.org/draft/2020-12/schema'
ID = 'urn:oteryn:monster-authoring:candidate:1'
DEPS_ID = 'urn:oteryn:monster-dependencies:candidate:1'
MANIFEST_ID = 'urn:oteryn:monster-import-readiness:candidate:1'

def obj(props, required=(), **extra):
    return {'type':'object','additionalProperties':False,'properties':props,'required':list(required),**extra}
def array(item, minimum=0, unique=False, **extra):
    return {'type':'array','items':item,'minItems':minimum,**({'uniqueItems':True} if unique else {}),**extra}
def integer(minimum=0, maximum=None, **extra):
    return {'type':'integer','minimum':minimum,**({'maximum':maximum} if maximum is not None else {}),**extra}
def text(minimum=1, **extra): return {'type':'string','minLength':minimum,**extra}
def enum(*values): return {'enum':list(values)}
def use(name): return {'$ref':'#/$defs/'+name}
def yes(field): return {'properties':{field:{'const':True}},'required':[field]}
def forbid(*names): return {'not':{'anyOf':[{'required':[n]} for n in names]}}
def reference(family):
    return obj({'family':{'const':family},'key':use('key'),'revision':use('revision')},('family','key','revision'))

d={}
d['key']=text(pattern=r'^[a-z0-9_.-]+:[a-z0-9_.:/-]+$',maxLength=512,
              description='Namespaced authoring key. Native ProductionKey validation still applies at integration.')
d['revision']=text(pattern=r'^[A-Za-z0-9_.:-]+$',maxLength=512)
d['identity']=obj({'key':use('key'),'revision':use('revision')},('key','revision'))
for f in ('Creature','Presentation','Behavior','Loot','Ability','Effect','Formula','Item','Document','Interaction','Encounter'):
    d[f+'Ref']=reference(f)
d['bool']={'type':'boolean'}
d['percent']={'type':'number','minimum':0,'maximum':100,
    'description':'Percent 0..100; 0.19 means 0.19%. At most 4 decimal places (0.0001% = 1 ppm), enforced by '
                  'validate_monster.py so that binary-float JSON Schema validators accept valid values. '
                  'Native ppm = percent * 10000, which must be an exact integer.'}
d['ms']=integer(1,description='Positive duration/interval in milliseconds; no implicit source-unit conversion.')
d['ratio']=obj({'numerator':{'type':'integer'},'denominator':integer(1)},('numerator','denominator'),
    description='Exact rational number in lowest terms (gcd(|numerator|, denominator) = 1, so zero is 0/1), matching '
                'WorldProject/v2 validate_v2_ratio. Percent values represent percentage points, not fractions of one.')
d['nonnegativePercent']=copy.deepcopy(d['ratio']);d['nonnegativePercent']['properties']['numerator']=integer()
d['nonnegativeRatio']=copy.deepcopy(d['nonnegativePercent']);d['nonnegativeRatio']['description']='Exact nonnegative dimensionless ratio.'
d['damageType']=enum('physical','energy','earth','fire','life_drain','mana_drain','drowning','ice','holy','death','agony','neutral','untyped','healing')
d['conditionType']=text(pattern=r'^[a-z][a-z0-9_]*$',description='Canonical condition key; source aliases require an explicit import mapping.')
d['resistance']=obj({'damage_type':use('damageType'),'reduction_percent':use('ratio')},('damage_type','reduction_percent'))
d['damageResponse']=obj({'damage_type':use('damageType'),'percent':use('nonnegativePercent')},('damage_type','percent'))
d['stats']=obj({
    'max_health':integer(1),'initial_health':integer(1),'experience':integer(),'speed':integer(),
    'armor':integer(),'defense':integer(),'mitigation_percent':use('nonnegativePercent'),
    'critical_chance_percent':use('percent')},('max_health','initial_health','experience','speed','armor','defense','critical_chance_percent'))
d['familiar']=obj({'vocation':text(),'summon_ability':use('AbilityRef'),'duration_ms':use('ms'),
    'mana_cost':integer(),'owner_speed_bonus':{'type':'integer'}},('vocation','summon_ability','duration_ms','mana_cost'))
d['summoning']=obj({'summonable':use('bool'),'mana_cost':integer(),'convinceable':use('bool'),
    'is_familiar':use('bool'),'familiar':use('familiar')},('summonable','convinceable','is_familiar'),allOf=[
    {'if':{'anyOf':[yes('summonable'),yes('convinceable')]},'then':{'required':['mana_cost']},'else':forbid('mana_cost')},
    {'if':yes('is_familiar'),'then':{'required':['familiar']},'else':forbid('familiar')}])
d['bestiary']=obj({'class':text(),'taxonomy':text(),'difficulty':text(),'occurrence':text(),
    'stars':integer(0,5),'locations':text(description='Original editorial location text, not world spawn coordinates.'),
    'notes':text(description='Optional Oteryn-authored Bestiary narrative; separate from source Locations, inspection text and library Documents.'),
    'kill_thresholds':array(integer(1),3,False,maxItems=3),'charm_points':integer(0,65535)},
    ('class','taxonomy','difficulty','occurrence','kill_thresholds','charm_points'))
d['bosstiary']=obj({'category':text(),'prowess_kills':integer(1),'expertise_kills':integer(1),
    'mastery_kills':integer(1),'prowess_points':integer(0,65535),'expertise_points':integer(0,65535),'mastery_points':integer(0,65535)},
    ('category','prowess_kills','expertise_kills','mastery_kills','prowess_points','expertise_points','mastery_points'),
    description='Boss points are awarded per reached stage (io_bosstiary.hpp levelInfos), not once per boss.')
d['creature']=obj({
    'identity':use('identity'),'display_name':text(),
    'name_forms':obj({'article':text(0),'plural':text()}),
    'inspection':obj({'description':text()},('description',)),
    'stats':use('stats'),'resistances':array(use('resistance')),
    'immunities':obj({'damage_types':array(use('damageType'),unique=True),'conditions':array(use('conditionType'),unique=True)},('damage_types','conditions')),
    'flags':obj({k:use('bool') for k in ('attackable','illusionable','health_hidden')},('attackable','illusionable','health_hidden')),
    'summoning':use('summoning'),'bestiary':use('bestiary'),'bosstiary':use('bosstiary'),
    'corpse_item':use('ItemRef'),'soul_core_item':use('ItemRef'),
    'death_residue':obj({'item':use('ItemRef'),'fluid_type':text(pattern=r'^[a-z][a-z0-9_]*$')},('item',),
        description='Item placed on death beside the corpse; fluid_type distinguishes one splash Item filled with blood, slime, etc. Omit when the race leaves none.'),
    'encyclopedia':obj({'description_document':use('DocumentRef')},('description_document',)),
    'presentation':use('PresentationRef'),'behavior':use('BehaviorRef'),'loot':use('LootRef'),
    'damage_reflection':array(use('damageResponse')),'healing_from_damage':array(use('damageResponse')),
    'system_eligibility':obj({k:use('bool') for k in ('prey','exclusive_prey','forge','reward_boss')},('prey','exclusive_prey','forge','reward_boss')),
    'spawn_eligibility':obj({'period':enum('all','day','night'),'ignore_period_underground':use('bool'),
                            'blocked_by_nearby_players':use('bool')},('period','ignore_period_underground','blocked_by_nearby_players')),
    'reward_encounter':use('EncounterRef')},
    ('identity','display_name','inspection','stats','resistances','immunities','flags','summoning','presentation','behavior','damage_reflection','healing_from_damage'))
d['schedule']=obj({'ability':use('AbilityRef'),'interval_ms':use('ms'),'chance_percent':use('percent'),
    'magnitude':obj({'minimum':integer(),'maximum':integer()},('minimum','maximum'),
                    description='D11: this monster\'s damage/heal magnitude for effects whose formula is caster_magnitude.'),
    'range_tiles':integer(description='D11: this monster\'s cast range for the shared Ability.')},('ability','interval_ms','chance_percent'))
d['voices']=obj({'interval_ms':use('ms'),'chance_percent':use('percent'),
    'entries':array(obj({'text':text(),'mode':enum('say','yell')},('text','mode')),1)},('interval_ms','chance_percent','entries'))
d['behavior']=obj({
    'identity':use('identity'),
    'movement':obj({'can_walk':use('bool'),'pass_through':use('bool'),'pushable':use('bool'),'push_items':use('bool'),'push_creatures':use('bool'),
        'field_permissions':obj({k:use('bool') for k in ('energy','fire','poison')},('energy','fire','poison'))},('can_walk','pass_through','pushable','push_items','push_creatures','field_permissions')),
    'targeting':obj({'hostile':use('bool'),'can_target':use('bool'),'sense_invisible':use('bool'),
        'target_distance_tiles':integer(),'static_attack_chance_percent':use('percent'),
        'change_target':obj({'interval_ms':integer(0,description='0 disables timed target changes; a nonzero chance still lets the monster '
                                                                'switch to an opponent that blocks its path to the current target.'),
                             'chance_percent':use('percent')},('interval_ms','chance_percent')),
        'strategy_weights':obj({k:integer() for k in ('nearest','damage','health','random')},('nearest','damage','health','random')),
        'flee_health':integer()},('hostile','can_target','sense_invisible','target_distance_tiles','static_attack_chance_percent','flee_health')),
    'attacks':array(use('schedule')),'defenses':array(use('schedule')),'voices':use('voices'),
    'summons':obj({'max_summons':integer(1),'entries':array(obj({'creature':use('CreatureRef'),
        'interval_ms':use('ms'),'chance_percent':use('percent'),'count':integer(1)},('creature','interval_ms','chance_percent','count')),1)},('max_summons','entries')),
    'periodic_audio':obj({'interval_ms':use('ms'),'chance_percent':use('percent'),'cue_ids':array(text(),1,True)},('interval_ms','chance_percent','cue_ids')),
    'faction_and_preferences':obj({'faction':text(),'enemy_factions':array(text(),unique=True),'prefer_player':use('bool'),
        'prefer_master':use('bool')},('faction','enemy_factions','prefer_player','prefer_master')),
    'event_bindings':array(obj({'event':enum('think','appear','disappear','move','say','attacked_by_player','spawn','death','drop_loot'),
        'interaction':use('InteractionRef')},('event','interaction')))},('identity','movement','targeting','attacks','defenses','event_bindings'))
d['assetBinding']=use('key')
d['presentation']=obj({
    'identity':use('identity'),
    'appearance':obj({'asset_binding':use('assetBinding'),
        'selection':{**enum('owner_familiar_look','invisible'),'description':'D16: owner_familiar_look shows the familiar look '
            'selected by the owner; asset_binding is the default look for a character without a selection. D24: invisible means '
            'the creature has no appearance at all (Canary lookType 0 without lookTypeEx) and forbids asset_binding.'},
        'palette_bindings':array(obj({'slot':enum('head','body','legs','feet','mount_head','mount_body','mount_legs','mount_feet'),'palette_binding':use('assetBinding')},('slot','palette_binding'))),
        'attachment_bindings':array(obj({'slot':enum('addon','mount','familiar','wing'),'asset_binding':use('assetBinding')},('slot','asset_binding'))),
        'visual_effect_bindings':array(obj({'slot':enum('aura','effect','shader'),'asset_binding':use('assetBinding')},('slot','asset_binding')))},
        ('palette_bindings','attachment_bindings','visual_effect_bindings'),allOf=[
        {'if':{'properties':{'selection':{'const':'invisible'}},'required':['selection']},
         'then':{**forbid('asset_binding'),'properties':{'palette_bindings':{'maxItems':0},'attachment_bindings':{'maxItems':0}}},
         'else':{'required':['asset_binding']}}]),
    'light':obj({'level':integer(0,255),'color_binding':use('assetBinding')},('level',)),
    'audio':obj({'event_bindings':array(obj({'event':enum('cast','impact','death','periodic'),'cue_id':text(),
        'asset_binding':use('assetBinding')},('event','cue_id','asset_binding')))},('event_bindings',)),
    'variant_label':text(),'status_marker':enum('none','yellow','green','white','red','black','orange')},('identity','appearance','light','audio'))
d['lootEntry']=obj({'item':use('ItemRef'),'min_count':integer(),'max_count':integer(1),
    'probability_percent':use('percent'),'skip_later_same_item_after_success':{'type':'boolean',
        'description':'On a successful positive-quantity drop, suppress later entries with the same exact Item identity in this ordered table invocation. Zero quantity does not suppress; nested/separate calls have separate scope.'},
    'instance_attributes':obj({'charges':integer(),'fluid_type':text(),'text':text(0),'name':text(),'article':text(0),
        'attack':integer(),'defense':integer(),'extra_defense':{'type':'integer'},'armor':integer(),
        'shoot_range_tiles':integer(),'hit_chance_percent':integer(-100,100),'interaction':use('InteractionRef')},
        oneOf=[forbid('fluid_type'),{'required':['fluid_type'],**forbid('charges')}]),
    'contents_loot':use('LootRef')},('item','min_count','max_count','probability_percent','skip_later_same_item_after_success'))
d['loot']=obj({'identity':use('identity'),'algorithm':enum('IndependentBernoulli'),
    'entries':array(use('lootEntry'))},('identity','algorithm','entries'),
    description='Only the source-supported independent base table is in this schema. Other native algorithms retain their own accepted contracts.')
d['areaMatrix']=array(text(pattern=r'^[.xcC]+$'),1,
    description="Rows of cells: '.' not hit, 'x' hit, 'c' centre not hit, 'C' centre hit; rectangular, exactly one centre.")
d['area']=obj({'length_tiles':integer(1),'spread_tiles':integer(),'radius_tiles':integer(),
    'matrix':obj({'north':use('areaMatrix'),'diagonal':use('areaMatrix')},('north',),
        description='D12: authored facing north (diagonal: facing north-west) and rotated to the direction from the centre to '
                    'the target (combat.cpp AreaCombat::getArea); tiles need a clear line of sight from the caster.')},oneOf=[
    {'required':['length_tiles','spread_tiles'],**forbid('radius_tiles','matrix')},
    {'required':['radius_tiles'],**forbid('length_tiles','spread_tiles','matrix')},
    {'required':['matrix'],**forbid('length_tiles','spread_tiles','radius_tiles')}])
d['ability']=obj({'identity':use('identity'),'kind':enum('melee','spell'),'range_tiles':integer(),'needs_target':use('bool'),'needs_direction':use('bool'),
    'area':use('area'),'effects':array(use('EffectRef'),1),'audio':obj({'cast_cue':text(),'impact_cue':text()}),
    'variants':array(use('AbilityRef'),2,description='D12: each cast runs one variant picked uniformly; variants have no variants.'),
    'path_requirement':obj({'max_search_tiles':integer(1),'clear_sight':use('bool')},('max_search_tiles','clear_sight'),
        description='D18: the cast needs a walking path to its target found within max_search_tiles (and a clear line of sight '
            'when clear_sight); without one the cast fails and nothing happens.'),
    'chain':obj({'max_targets':integer(1),'range_tiles':integer(1),'backtracking':use('bool'),'chain_asset_binding':use('assetBinding'),
                 'target_filter':enum('players','ranged_monsters'),
                 'shape':{**enum('sequential','fork'),'description':'sequential (default): each jump starts at the last creature '
                     'hit; fork: every further creature is chosen within range_tiles of the first creature.'},
                 'initial_range_tiles':integer(1,description='Search radius for the first creature of a cast without a target; '
                     'absent means range_tiles.'),
                 'damage_step_percent':integer(-100,100,description='Per-step change of the rolled value: step i gets '
                     '(100 + i * damage_step_percent)%, at least 0; absent means 0.')},
                ('max_targets','range_tiles','backtracking'),description='D12: one cast hits the first creature and up to max_targets '
                'further creatures (Canary: the chain value callback count). target_filter players skips every creature that is not a '
                'player (the players-only chain picker); ranged_monsters keeps monsters that are not summons, not reward bosses and '
                'prefer a target distance above 1. Spell chains: OTERYN_SPELL_CHAIN_BEHAVIOUR_CANDIDATE_V1.md.'),
    'encounter':{**use('EncounterRef'),'description':'D45: the cast has no effect of its own; its effect is the ability_cast rule '
        'of this Encounter, which owns the fight state the script reads (summon counts, counters, arena tiles).'}},
    ('identity','kind','range_tiles','needs_target','needs_direction'),
    oneOf=[{'required':['effects'],**forbid('variants','encounter')},{'required':['variants'],**forbid('effects','area','chain','encounter')},
           {'required':['encounter'],'properties':{'kind':{'const':'spell'}},**forbid('effects','variants','area','chain')}],
    description='For area casts, center precedence is required target position, facing-adjacent position when needs_direction, then caster position. Native execution and no-area target selection require separate qualification.')
d['initialTick']=obj({'mode':enum('automatic','fixed'),'amount':integer(1)},('mode',),allOf=[
    {'if':{'properties':{'mode':{'const':'fixed'}},'required':['mode']},'then':{'required':['amount']},'else':forbid('amount')}])
d['damageOverTime']=obj({
    'fixed_ticks':array(obj({'count':integer(1),'interval_ms':use('ms'),'amount':integer(1)},('count','interval_ms','amount')),1,
        description='D12: explicit ticks in order (Condition:addDamage rounds, time, value).'),
    'total_damage_range':obj({'minimum':integer(0,description='0 is allowed: the total is drawn uniformly from the range and a '
                                                               'zero draw means the condition does not start (condition.cpp ConditionDamage::init).'),
                              'maximum':integer(1)},('minimum','maximum')),
    'tick_interval_ms':use('ms'),'initial_tick':use('initialTick'),
    'geometric':obj({'base_range':obj({'minimum':integer(1),'maximum':integer(1)},('minimum','maximum')),'factor':use('ratio'),
        'tick_counts':array(integer(1),1,True),'tick_interval_ms':use('ms')},('base_range','factor','tick_counts','tick_interval_ms'),
        description='D21: each cast draws an integer base uniformly from base_range and a tick count uniformly from tick_counts; '
            'tick k (from 0) deals the base multiplied k times by factor in double precision, truncated toward zero.'),
    'tick_profile':enum('decreasing','fixed','geometric'),'first_tick':enum('after_interval','immediate')},
    ('tick_profile','first_tick'),allOf=[
    {'if':{'properties':{'tick_profile':{'const':'fixed'}},'required':['tick_profile']},
     'then':{'required':['fixed_ticks'],**forbid('total_damage_range','tick_interval_ms','initial_tick','geometric')}},
    {'if':{'properties':{'tick_profile':{'const':'geometric'}},'required':['tick_profile']},
     'then':{'required':['geometric'],**forbid('total_damage_range','tick_interval_ms','initial_tick','fixed_ticks')}},
    {'if':{'properties':{'tick_profile':{'const':'decreasing'}},'required':['tick_profile']},
     'then':{'required':['total_damage_range','tick_interval_ms','initial_tick'],**forbid('fixed_ticks','geometric')}}],
    description='Nominal source damage budget, not per-tick magnitude or a promise of exact summed damage. Native schedule parity requires separate qualification.')
d['condition']=obj({'type':use('conditionType'),'lifetime':enum('fixed_duration','damage_schedule'),
    'damage_over_time':use('damageOverTime'),'speed_formula':use('FormulaRef'),
    'attribute_modifiers':array(obj({'attribute':text(pattern=r'^[a-z][a-z0-9_]*$'),'mode':enum('percent_of_base','add'),
        'value':{'type':'integer'}},('attribute','mode','value')),1,
        description='D12: percent_of_base sets the attribute to value% of its base; add adds value.'),
    'light':obj({'level':integer(1,255),'color':integer(0,255)},('level','color'),
        description='S17: light radius level and 8-bit light colour of a light condition.'),
    'regeneration':obj({'health_gain':integer(1),'health_interval_ms':use('ms'),'mana_gain':integer(1),
                        'mana_interval_ms':use('ms')},(),minProperties=2,
        dependentRequired={'health_gain':['health_interval_ms'],'health_interval_ms':['health_gain'],
                           'mana_gain':['mana_interval_ms'],'mana_interval_ms':['mana_gain']},
        description='S17: hitpoints and/or mana restored every interval while the regeneration condition lasts.'),
    'buff_spell':{'type':'boolean','description':'S17: the condition is a spell buff (Canary CONDITION_PARAM_BUFF_SPELL).'}},
    ('type','lifetime'),allOf=[
    {'if':{'properties':{'lifetime':{'const':'damage_schedule'}},'required':['lifetime']},
     'then':{'required':['damage_over_time'],**forbid('speed_formula','light','regeneration','buff_spell')},'else':forbid('damage_over_time')},
    {'if':{'properties':{'type':{'const':'light'}},'required':['type']},'then':{'required':['light']},'else':forbid('light')},
    {'if':{'properties':{'type':{'const':'regeneration'}},'required':['type']},'then':{'required':['regeneration']},
     'else':forbid('regeneration')}])
d['effect']=obj({'identity':use('identity'),'operation':enum('damage','heal','condition','appearance_transform','create_item','presentation_only','remove_condition',
                     'remove_items','summon_creature'),
    'damage_type':use('damageType'),'formula':use('FormulaRef'),'duration_ms':use('ms'),'condition':use('condition'),
    'mitigated_by':array(enum('armor','shield'),1,True,description='Target defences that reduce this damage (Canary/Crystal '
        'blockedByArmor/blockedByShield); absent means neither. The reduction formula itself is a world combat rule.'),
    'appearance_transform':obj({'creature':use('CreatureRef'),'item':use('ItemRef')},oneOf=[
        {'required':['creature'],**forbid('item')},{'required':['item'],**forbid('creature')}]),
    'created_item':use('ItemRef'),
    'affects':obj({'kind':enum('masterless_monsters','non_player_side','player_side','players','named_creatures'),
        'creatures':array(use('CreatureRef'),1,True),'top_creature_only':use('bool'),'excludes_caster_name':use('bool'),
        'includes_caster':use('bool')},
        ('kind','top_creature_only','excludes_caster_name','includes_caster'),allOf=[
        {'if':{'properties':{'kind':{'const':'named_creatures'}},'required':['kind']},'then':{'required':['creatures']},'else':forbid('creatures')}],
        description='D18: which creatures in the ability area this damage/heal reaches instead of the combat targets. masterless_monsters = '
            'monsters without a master; non_player_side = neither a player nor a player-owned summon; player_side = a player or a '
            'player-owned summon; players = players only; named_creatures = those creatures. top_creature_only: only the top creature of each tile; excludes_caster_name: not '
            'creatures named like the caster; includes_caster: the caster is affected when it stands in the area.'),
    'removed_items':obj({'items':array(use('ItemRef'),1,True),'selection':enum('first_listed_per_tile','top_item_first_tile')},
        ('items','selection'),description='D18: remove_items over the ability area. first_listed_per_tile removes on every tile the first '
            'listed item present; top_item_first_tile removes the top item of the first tile where it is listed, then stops. Tiles are '
            'scanned column by column, west to east, each column north to south (Canary destroy_magic_walls.lua).'),
    'summon':obj({'creatures':array(use('CreatureRef'),1,True),'count_mode':enum('fill_to_limit','fixed'),'count':integer(1),
        'only_below_summons':integer(1),'owned':use('bool'),'max_offset_tiles':integer(0)},
        ('creatures','count_mode','count','only_below_summons','owned','max_offset_tiles'),
        description='D18: summon_creature. Nothing is created unless the caster has fewer than only_below_summons summons; then '
            'fill_to_limit creates count minus current summons, fixed creates count. Each creature is picked uniformly from creatures and '
            'requested at the caster position offset by up to max_offset_tiles per axis; owned makes the caster its master.'),
    'removed_condition':{**use('conditionType'),'description':'remove_condition: every condition of this type on the target '
        'ends (Canary/Crystal COMBAT_PARAM_DISPEL, Creature::removeCombatCondition).'},
    'presentation':obj({'impact_asset_binding':use('assetBinding'),'projectile_asset_binding':use('assetBinding'),
        'path_asset_binding':{**use('assetBinding'),'description':'D18: shown on every tile of the walking path from the caster to '
            'the target before the effect applies.'}})},
    ('identity','operation'),allOf=[
    {'if':{'properties':{'operation':{'enum':['damage','heal']}},'required':['operation']},
     'then':{'required':['formula','damage_type'],**forbid('condition','appearance_transform','created_item')},
     'else':forbid('formula','damage_type')},
    {'if':{'properties':{'operation':{'const':'condition'}},'required':['operation']},
     'then':{'required':['condition'],**forbid('appearance_transform','created_item'),
             'allOf':[{'if':{'properties':{'condition':{'properties':{'lifetime':{'const':'damage_schedule'}}}}},
                       'then':forbid('duration_ms'),'else':{'required':['duration_ms']}}]},'else':forbid('condition')},
    {'if':{'properties':{'operation':{'const':'appearance_transform'}},'required':['operation']},
     'then':{'required':['appearance_transform','duration_ms'],**forbid('created_item')},'else':forbid('appearance_transform')},
    {'if':{'properties':{'operation':{'const':'create_item'}},'required':['operation']},
     'then':{'required':['created_item']},'else':forbid('created_item')},
    {'if':{'properties':{'operation':{'const':'presentation_only'}},'required':['operation']},
     'then':{'required':['presentation'],'properties':{'presentation':{'minProperties':1}},**forbid('duration_ms')}},
    {'if':{'properties':{'operation':{'const':'damage'}},'required':['operation']},'then':{},'else':forbid('mitigated_by')},
    {'if':{'properties':{'operation':{'enum':['damage','heal']}},'required':['operation']},'then':{},'else':forbid('affects')},
    {'if':{'properties':{'operation':{'const':'remove_items'}},'required':['operation']},
     'then':{'required':['removed_items'],**forbid('duration_ms')},'else':forbid('removed_items')},
    {'if':{'properties':{'operation':{'const':'summon_creature'}},'required':['operation']},
     'then':{'required':['summon'],**forbid('duration_ms')},'else':forbid('summon')},
    {'if':{'properties':{'operation':{'const':'remove_condition'}},'required':['operation']},
     'then':{'required':['removed_condition'],**forbid('duration_ms')},'else':forbid('removed_condition')}])
d['formula']=obj({'identity':use('identity'),'kind':enum('range','melee_attack_skill','speed_modifier','caster_magnitude'),
    'magnitude':obj({'minimum':integer(),'maximum':integer()},('minimum','maximum')),
    'melee':obj({'attack':integer(),'skill':integer()},('attack','skill')),
    'speed':obj({'minimum_multiplier':use('nonnegativeRatio'),'minimum_offset':{'type':'integer'},
        'maximum_multiplier':use('nonnegativeRatio'),'maximum_offset':{'type':'integer'}},
        ('minimum_multiplier','minimum_offset','maximum_multiplier','maximum_offset'))},('identity','kind'),allOf=[
    {'if':{'properties':{'kind':{'const':'range'}},'required':['kind']},'then':{'required':['magnitude'],**forbid('melee','speed')}},
    {'if':{'properties':{'kind':{'const':'melee_attack_skill'}},'required':['kind']},'then':{'required':['melee'],**forbid('magnitude','speed')}},
    {'if':{'properties':{'kind':{'const':'speed_modifier'}},'required':['kind']},'then':{'required':['speed'],**forbid('magnitude','melee')}},
    {'if':{'properties':{'kind':{'const':'caster_magnitude'}},'required':['kind']},'then':forbid('magnitude','melee','speed')}],
    description='caster_magnitude (D11): the magnitude comes from the casting monster\'s schedule entry (Monster::getCombatValues).')
d['document']=obj({'identity':use('identity'),
    'document_type':enum('Book','Letter','Note','Diary','Report','Scroll','Parchment','Tablet','Inscription','Notice','Other'),
    'title':text(),'author':text(),'language':text(pattern=r'^[a-z]{2,3}(?:-[A-Za-z0-9]{2,8})*$'),
    'content':array(text(),1)},('identity','document_type','title','language','content'),
    description='Original paragraphs; no translation, correction, or paraphrase during import. Partial source text remains blocked in the import manifest.')
d['temporal']=obj({'duration_ms':use('ms'),'decay_action':enum('none','transform','remove'),
    'decay_target':use('ItemRef'),'stop_duration':use('bool')},('decay_action','stop_duration'),allOf=[
    {'if':{'properties':{'decay_action':{'const':'transform'}},'required':['decay_action']},
     'then':{'required':['duration_ms','decay_target']}},
    {'if':{'properties':{'decay_action':{'const':'remove'}},'required':['decay_action']},
     'then':{'required':['duration_ms'],**forbid('decay_target')}},
    {'if':{'properties':{'decay_action':{'const':'none'}},'required':['decay_action']},
     'then':forbid('duration_ms','decay_target')}])
d['item']=obj({'identity':use('identity'),
    'presentation':obj({'name':text(),'article':text(0),'description':text(),'asset_binding':use('assetBinding')},('name','asset_binding')),
    'classification':obj({'is_corpse':use('bool')},('is_corpse',)),
    'physical':obj({'weight_centioz':integer(0,4294967295,description='Proposal authoring unit: 0.01 oz. Not inferred from absent XML attributes.'),
        'movable':use('bool'),'pickupable':use('bool')},('weight_centioz','movable','pickupable')),
    'collision':obj({k:use('bool') for k in ('block_walk','block_projectiles','block_pathfinding')},('block_walk','block_projectiles','block_pathfinding')),
    'container':obj({'capacity':integer(1)},('capacity',)),
    'temporal':use('temporal'),'fluid':obj({'fluid_type':text(),'fluid_source':text()})},
    ('identity','presentation','classification','physical','collision','temporal'),
    description='Verification projection of canonical Item capabilities. Not an independent Item truth authority or full native Item serialization. ItemRef must resolve to the canonical Item catalogue before runtime admission.')

main={'$schema':DIALECT,'$id':ID,'title':'Monster authoring schema candidate v1 — structural validation',
    'description':'CANDIDATE authoring schema (docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md). '
                  'Not a WorldProject/v2 contract or runtime activation; semantic rules live in validate_monster.py.',
    **obj({'creature':use('creature'),'behavior':use('behavior'),'presentation':use('presentation'),'loot':use('loot')},
          ('creature','behavior','presentation')),'$defs':d}
def external(name): return {'$ref':ID+'#/$defs/'+name}
deps={'$schema':DIALECT,'$id':DEPS_ID,'title':'Direct monster dependencies authoring schema candidate v1',
    **obj({'abilities':array(external('ability')),'effects':array(external('effect')),'formulas':array(external('formula')),
        'documents':array(external('document')),'items':array(external('item')),'loot_tables':array(external('loot'))},
        ('abilities','effects','formulas','documents','items','loot_tables'))}
manifest={'$schema':DIALECT,'$id':MANIFEST_ID,'title':'Monster import disposition manifest candidate v1',
    **obj({'sources':array({'oneOf':[
            obj({'repository':text(),'revision':text(pattern=r'^[a-f0-9]{40}$')},('repository','revision'),
                description='Git source pinned to an exact commit.'),
            obj({'kind':{'const':'mediawiki'},'api':text(pattern=r'^https://'),'title':text(),'page_id':integer(1),
                 'revision_id':integer(1),'content_sha256':text(pattern=r'^[a-f0-9]{64}$')},
                ('kind','api','title','page_id','revision_id','content_sha256'),
                description='MediaWiki page pinned to one immutable revision id and the SHA-256 of that revision wikitext (D15). '
                            'source_file is the page title and source_line the wikitext line.'),
            obj({'kind':{'const':'official_capture'},'url':text(pattern=r'^https://'),'title':text(),
                 'captured':text(pattern=r'^[0-9]{4}-[0-9]{2}-[0-9]{2}$'),'content_sha256':text(pattern=r'^[a-f0-9]{64}$')},
                ('kind','url','title','captured','content_sha256'),
                description='Spell S15: a dated capture of an official game-publisher page (tibia.com) that has no revision id; '
                            'content_sha256 is that of the captured facts. source_file is the entry title.')]},1),
        'entries':array(obj({'source_index':integer(),'source_file':text(),'source_line':integer(1),'source_field':text(),
            'kind':enum('field','dependency','script','original_text'),
            'status':enum('mapped','metadata_only','resolved_native_behavior','approved_omission','unsupported_source_field','unresolved_semantics','unresolved_dependency','partial_text'),
            'destination':text(),'resolution':text()},('source_index','source_file','source_line','source_field','kind','status'),allOf=[
            {'if':{'properties':{'status':{'enum':['mapped','resolved_native_behavior']}},'required':['status']},'then':{'required':['destination','resolution']}},
            {'if':{'properties':{'status':{'enum':['metadata_only','approved_omission']}},'required':['status']},'then':{'required':['resolution']}}]),1)},('sources','entries'))}

def dump(name,value): (ROOT/name).write_text(json.dumps(value,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
def template(s):
    if '$ref' in s:
        return template(d[s['$ref'].split('/')[-1]])
    if s.get('type')=='object':return {k:template(v) for k,v in s['properties'].items()}
    if s.get('type')=='array':return [template(s['items'])]
    return None

def build():
    dump('monster.schema.json',main);dump('monster-dependencies.schema.json',deps);dump('monster-import-readiness.schema.json',manifest)
    dump('monster-template.json',template(main));dump('monster-dependencies-template.json',template(deps))
    print('Generated 3 schemas and 2 empty templates.')

if __name__=='__main__': build()
