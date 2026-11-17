"""M25: only the two selected authored Galthen kills, on existing QuestState."""
import copy

QUEST = 'oteryn:quest.adventures_of_galthen_quest'
TRACK = 'oteryn:quest-progress/chosen-source/adventures_of_galthen_quest/'
TRANSITION = 'oteryn:quest-transition/chosen-source/adventures_of_galthen_quest/'
BASIS = 'CHOSEN_OTERYN_APPROXIMATION'
SOURCE_KEY = 'oteryn:chosen-source-stage/adventures_of_galthen_quest/'
ROWS = [('s5', 's4', 'Ahau', 'oteryn:creature.ahau'),
        ('s8', 's7', 'Mitmah Vanguard', 'oteryn:creature.mitmah_vanguard')]


def append_selected(definition, lowered):
    if definition['identity']['key'] != QUEST:
        return lowered
    supplement = definition.get('oteryn_recipe', {})
    if supplement.get('profile') != 'chosen_source_completion_v1':
        raise ValueError('Galthen chosen source recipe profile changed')
    recipe = supplement['payload']['recipe']
    stages = {stage['key']: stage for stage in recipe['stages']}
    if len(stages) != len(recipe['stages']):
        raise ValueError('duplicate Galthen authored stage')
    for key, kind, count, targets, next_stage in [
        ('s4','use',1,['Key Intihuatac','Iksupan'],['s5']),
        ('s5','kill',1,['Ahau'],['s6']),
        ('s7','use',1,['Pilchi with Paint','Ceremonial Brush'],['s8']),
        ('s8','kill',1,['Mitmah Vanguard'],['s9']),
        ('s9','complete',1,['Kesar the Younger','Galthen and the City of Gold'],[]),
    ]:
        stage = stages[key]
        if (stage.get('basis'),stage.get('kind'),stage.get('count'),stage.get('targets'),stage.get('next')) != (BASIS,kind,count,targets,next_stage):
            raise ValueError('Galthen authored predecessor/boss/report stage changed: '+key)
    value = copy.deepcopy(lowered)
    selected = {key for stage,predecessor,_,_ in ROWS for key in (stage,predecessor)}
    existing = {track['key'] for track in value['tracks']}
    if any(TRACK+key in existing for key in selected):
        raise ValueError('duplicate selected Galthen native stage track')
    for key in sorted(selected):
        value['tracks'].append({'key':TRACK+key, 'quest':QUEST, 'initial':0, 'min':0, 'max':1,
            'bounds_basis':'EXPLICIT_CHOSEN_STAGE_OCCURRENCE_COUNT', 'source_key':SOURCE_KEY+key})
    for stage, predecessor, _, creature in ROWS:
        if any(t['key'] == TRANSITION+stage for t in value['transitions']):
            raise ValueError('duplicate selected Galthen native kill transition')
        value['transitions'].append({
            'key':TRANSITION+stage, 'quest':QUEST, 'completes':False, 'requested_by':None,
            'effects':[
                {'track':TRACK+predecessor,'from':{'op':'EQ','value':1},'from_exact':True,'effect':{'kind':'SET','value':1}},
                {'track':TRACK+stage,'from':{'op':'LT','value':1},'from_exact':True,'effect':{'kind':'ADD','value':1}},
            ],
            'source':{'basis':BASIS, 'owner':'native_kills', 'key':stage, 'callback':None,
                'chosen_stage':copy.deepcopy(stages[stage]),
                'native_dispatch_binding':{'owner':'native_kills','creature':{'family':'Creature','key':creature,'revision':'definition-r1'}},
                'source_equivalence':False},
        })
    value['tracks'].sort(key=lambda t:t['key'])
    value['transitions'].sort(key=lambda t:t['key'])
    # Source completion and all other stages remain unchanged and unactivated.
    return value


def bind_selected_plan(records):
    """Expose only the existing native-death consumer's two compiled bindings."""
    owners = [row for row in records if row['quest'] == QUEST]
    if len(owners) != 1:
        raise ValueError('one Galthen completion binding owner')
    owner = owners[0]
    for stage, predecessor, _, creature in ROWS:
        rows = [row for row in owner['stages'] if row['stage_key'] == stage]
        if len(rows) != 1 or rows[0]['quest_transition_key'] != TRANSITION+stage:
            raise ValueError('exact Galthen native stage binding missing or duplicated')
        row = rows[0]
        intent = row['event_identity_associations']
        if intent['kind'] != 'kill' or intent['count'] != 1:
            raise ValueError('Galthen native stage event/count changed')
        row['native_dispatch_binding'] = {
            'owner':'native_kills', 'creature':{'family':'Creature','key':creature,'revision':'definition-r1'},
            'transition':TRANSITION+stage, 'predecessor':{'track':TRACK+predecessor,'value':1},
        }
        # The candidate as a whole remains unactivated; this records the
        # exact production consumer without activating other recipe stages.
        row['runtime_enabled'] = False
    # Other stages, NPC requested-by and separate report/reward remain held.
