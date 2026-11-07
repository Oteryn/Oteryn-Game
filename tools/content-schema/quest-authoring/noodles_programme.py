"""Guaranteed personal King/Noodles programme, independent of world-event scheduling."""
import hashlib
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
QUEST='oteryn:quest.authored.noodles_is_gone_mini_world_change'
TRACK='oteryn:quest-progress/noodles/stage'

def extend_rewards(callbacks,greetings):
    path='content/quests/definitions/quests-00300-00351.json'
    raw=(ROOT/path).read_bytes()
    definition,=[r['definition'] for r in json.loads(raw)['records'] if r['definition']['identity']['key']==QUEST]
    recipe=definition['recipe']
    if len(recipe['stages'])!=4 or recipe['requirements']['premium'] or recipe['repeat']['kind']!='once':
        raise ValueError('NOODLES_SELECTED_RECIPE_DRIFT')
    source=f'oteryn:selected-recipe:{path}:/records/22/definition:sha256:{hashlib.sha256(raw).hexdigest()}'
    greetings.append(dict(npc='oteryn:npc.king_tibianus',greet='Welcome. Ask about Noodles and his collar.',farewell='Farewell. Keep my dog safe.',walkaway='Return when you have news of Noodles.',sources=[source]))
    def add(name,npc,words,before,after,reply,give=None,**extra):
        callbacks.append(dict(key='oteryn:quest-reward/noodles-'+name,quest=QUEST,npc='oteryn:npc.'+npc,keywords=words,topic=0,next_topic=0,
            storage=[dict(track=TRACK,minimum=before,maximum=before,set=after)],take=[],give=give,outfit=None,achievement=None,reply=[reply],sources=[source],**extra))
    add('collar','king_tibianus',['noodles','collar','mission'],-1,0,
        'Take this collar. Find Noodles in the courtyard, use the collar on him, then report to me.',give=dict(item='oteryn:item.tibia.i14156',count=1,contents=[]))
    add('recovered','noodles',['collar'],1,2,'Noodles is safe. Report to King Tibianus.',carried_item='oteryn:item.tibia.i14156',carried_target_npc=True)
    callbacks[-1]['take']=[dict(item='oteryn:item.tibia.i14156',count=1)]
    add('report','king_tibianus',['report','reward'],2,3,'Noodles is safe. Here is your reward.',give=dict(item='oteryn:item.tibia.i3035',count=5,contents=[]),experience=1000,completes=True)
    callbacks[-1]['achievement']='oteryn:achievement/dog_sitter'

def extend_world(rows):
    from qengine_access import callback
    path='content/quests/definitions/quests-00300-00351.json';raw=(ROOT/path).read_bytes()
    row=callback('authored-noodles-find','authored.noodles_is_gone_mini_world_change','step_in','selected-personal-permanent-courtyard-variant',source_pin='oteryn-authored:',at=dict(x=32313,y=32178,floor=-6),appearance=464)
    row['storage']=[dict(track=TRACK,minimum=0,maximum=0,set=1)]
    region='content/world/placements/region-z06-x126-y125.b3'
    row['sources'] += [f'oteryn:selected-recipe:{path}:/records/22/definition:sha256:{hashlib.sha256(raw).hexdigest()}',f'oteryn:source-map:{region}:sha256:{hashlib.sha256((ROOT/region).read_bytes()).hexdigest()}']
    rows.append(row)
