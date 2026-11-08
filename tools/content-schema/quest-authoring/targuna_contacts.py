"""Selected first Targuna contact; later stages and rewards remain separately guarded."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
QUEST = 'oteryn:quest.targuna_quest'
TRACK = 'oteryn:quest-progress/targuna/stage'


def extend_rewards(callbacks, greetings):
    path = 'content/quests/definitions/quests-00200-00299.json'
    raw = (ROOT / path).read_bytes()
    definition = json.loads(raw)['records'][79]['definition']
    recipe = definition['oteryn_recipe']['payload']['recipe']
    if (definition['identity']['key'] != QUEST or recipe['requirements']['premium']
            or recipe['requirements']['min_level'] != 0
            or recipe['requirements']['prerequisites'] != []
            or recipe['repeat']['kind'] != 'once' or len(recipe['stages']) != 9
            or recipe['stages'][0]['kind'] != 'talk'
            or recipe['stages'][0]['targets'] != ['Sterling']):
        raise ValueError('TARGUNA_SELECTED_FIRST_CONTACT_DRIFT')
    source = f'oteryn:selected-recipe:{path}:/records/79/definition:sha256:{hashlib.sha256(raw).hexdigest()}'
    npc = 'oteryn:npc.sterling'
    greetings.append(dict(npc=npc, greet='Welcome. Ask about the lost treasure.',
        farewell='Farewell.', walkaway='Return when you are ready.', sources=[source]))
    callbacks.append(dict(key='oteryn:quest-reward/questpc5-targuna-sterling-accept',
        quest=QUEST, npc=npc, keywords=['mission', 'lost treasure'], topic=0, next_topic=0,
        storage=[dict(track=TRACK, minimum=-1, maximum=-1, set=0)],
        take=[], give=None, outfit=None, achievement=None,
        reply=['Ask Morla on Aragonia about the old treasure map, then return to me.'],
        sources=[source]))
