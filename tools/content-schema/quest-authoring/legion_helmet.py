"""Chosen Legion route contacts; original source claims and prerequisites retained."""
import hashlib
import json
from pathlib import Path
ROOT = Path(__file__).resolve().parents[3]

def extend(callbacks):
    path = 'content/quests/definitions/quests-00000-00099.json'
    raw = (ROOT/path).read_bytes()
    definition, = [r['definition'] for r in json.loads(raw)['records']
                   if r['definition']['identity']['key']=='oteryn:quest.newhaven_legion_helmet_quest']
    recipe = definition['oteryn_recipe']['payload']['recipe']
    if (recipe['requirements']['min_level'] != 0 or recipe['requirements']['prerequisites'] != []
            or recipe['stages'][0]['targets'] != ['Gustavo, The Guard','Viola']
            or recipe['reward_intents'][0]['item']['key'] != 'oteryn:item.tibia.i3374'):
        raise ValueError('LEGION_SELECTED_RECIPE_DRIFT')
    source = f'oteryn:selected-recipe:{path}:/records/62/definition:sha256:{hashlib.sha256(raw).hexdigest()}'
    for name,npc in [('gustavo','gustavo_the_guard'),('viola','viola')]:
        callbacks.append(dict(key='oteryn:quest-reward/legion-contact-'+name,
            quest='oteryn:quest.newhaven_legion_helmet_quest',npc='oteryn:npc.'+npc,
            keywords=['helmet','legion'],topic=0,next_topic=0,
            storage=[dict(track='oteryn:quest-progress/legion-helmet/'+name,minimum=-1,maximum=-1,set=1)],
            take=[],give=None,outfit=None,achievement=None,
            reply=['The helmet route leads through the training cave. Speak to both of us, descend to the equipment room, search the dead human, then return through the magic forcefield.'],
            sources=[source]))
