"""Source key sale for the authored Fibula path; no source sale limit is invented."""
import hashlib
import re
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]

PHASE = 'oteryn:quest-progress/fibula/supplement'


def append(callbacks, offers, inputs):
    path = 'data-global/npc/dermot.lua'
    raw = inputs[path]
    text = raw.decode()
    if (not re.search(r'removeMoneyBank\(2000\)', text)
            or not re.search(r'player:addItem\(2968,\s*1\)', text)
            or not re.search(r'key:setActionId\(3940\)', text)):
        raise ValueError('Fibula key sale source changed')
    digest = hashlib.sha256(raw).hexdigest()
    source = f'crystal:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:{path}:66-81:sha256:{digest}'
    for repeat in (False, True):
        # The first purchase activates only the optional chosen supplement.
        # Later source sales remain available before and after its completion.
        storage = dict(track=PHASE, minimum=0 if repeat else -1,
                       maximum=1 if repeat else -1, set=0)
        if repeat:
            storage['preserve'] = True
        callbacks.append(dict(
            key='oteryn:quest-reward/fibula-key-' + ('repeat' if repeat else 'first'),
            quest='oteryn:quest.deeper_fibula_quest', npc='oteryn:npc.dermot',
            keywords=['yes'], topic=2, next_topic=0, storage=[storage], take=[],
            fee=dict(gold=2000, include_bank=True),
            **({'repeatable': True} if repeat else {}),
            give=dict(item='oteryn:item.tibia.i2968', count=1, contents=[], key_number=3940),
            outfit=None, achievement=None, reply=['Here it is.'], sources=[source]))
    offers.append(dict(npc='oteryn:npc.dermot', keywords=['key'], topic=0,
                       next_topic=2, storage=[],
                       reply=['Do you want to buy the dungeon key for 2000 gold?']))

    # Terminal acknowledgement is an explicit Oteryn adaptation. Source chests
    # retain all loot; immutable canonical receipts prove they were actually used.
    recipe_path='tools/content-schema/quest-authoring/samples/completion242/recipes.json'
    raw_recipe=(ROOT/recipe_path).read_bytes()
    recipe,=[r['recipe'] for r in json.loads(raw_recipe)['records']
             if r['identity']['key']=='oteryn:quest.deeper_fibula_quest']
    definition_path='content/quests/definitions/quests-00000-00099.json'
    raw_definition=(ROOT/definition_path).read_bytes()
    definition,=[r['definition'] for r in json.loads(raw_definition)['records']
                 if r['definition']['identity']['key']=='oteryn:quest.deeper_fibula_quest']
    claims=definition['claims']
    if len(claims)!=6 or any(c['family']!='RewardClaim' for c in claims):
        raise ValueError('Fibula original six claim contract drift')
    sources=[f'owner:questpc6:chosen-recipe:{recipe_path}:6863-7011:sha256:{hashlib.sha256(raw_recipe).hexdigest()}',
             f'game:{definition_path}:1#/records/20/definition/claims:sha256:{hashlib.sha256(raw_definition).hexdigest()}']
    callbacks.append(dict(key='oteryn:quest-reward/fibula-complete',quest='oteryn:quest.deeper_fibula_quest',
        npc='oteryn:npc.dermot',keywords=['yes'],topic=3,next_topic=0,
        storage=[dict(track=PHASE,minimum=0,maximum=0,set=1),
                 dict(track='oteryn:quest-progress/fibula/cavern_kills',minimum=8,maximum=8,set=8,preserve=True)],
        completes=True,minimum_level=recipe['requirements']['min_level'],
        required_claims=sorted(c['key'] for c in claims),take=[],give=None,outfit=None,achievement=None,
        reply=['You have completed the deeper Fibula expedition. The recovered equipment is yours.'],sources=sources))
    for phase,topic,reply in [
        (-1,0,'Buy the dungeon key first. The deeper expedition is for adventurers of level 50 or higher.'),
        (0,3,'Defeat eight Dragons or Dragon Lords in the final cavern and recover every equipment cache. Are you ready to report your expedition?'),
        (1,0,'Your expedition is complete. You can still buy another dungeon key.'),
    ]:
        offers.append(dict(npc='oteryn:npc.dermot',keywords=['expedition'],topic=0,next_topic=topic,
            storage=[dict(track=PHASE,minimum=phase,maximum=phase,set=phase,preserve=True)],reply=[reply]))
