"""Source key sale for the authored Fibula path; no source sale limit is invented."""
import hashlib
import re
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]

PHASE = 'oteryn:quest-progress/fibula/supplement'
DONATIONS = 'oteryn:quest-progress/fibula/simon-donations'


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

def append_simon(callbacks, offers, tracks, inputs):
    path = 'data-global/npc/simon_the_beggar.lua'
    text = inputs[path].decode()
    fragments = ('MsgContains(message, "help")', 'removeMoneyBank(100)',
                 'removeMoneyBank(500)', 'removeMoneyBank(200)',
                 'player:addItem(2968, 1)', 'key:setActionId(3940)')
    if not all(fragment in text for fragment in fragments):
        raise ValueError('Fibula Simon key sale source changed')
    source = ('crystal:201189f871d23597b82caf88fd611454a2ddefb8:'
              f'{path}:237-290:sha256:7ad2665ab2a053412536f0dd15762c98fb07122f0db20e020180fecd89003e74')
    quest = 'oteryn:quest.deeper_fibula_quest'
    npc = 'oteryn:npc.simon_the_beggar'
    prompt = 'Could you spare 100 gold?'
    tracks.append(dict(key=DONATIONS, quest=quest, initial=0, min=0, max=2,
                       writable=True, sources=[source]))

    def stage(minimum, maximum, value):
        return dict(track=DONATIONS, minimum=minimum, maximum=maximum, set=value)

    def reward(name, keyword, topic, next_topic, storage, reply, **terms):
        callbacks.append(dict(key=f'oteryn:quest-reward/fibula-simon-{name}',
            quest=quest, npc=npc, keywords=[keyword], topic=topic, next_topic=next_topic,
            storage=storage, take=[], outfit=None, achievement=None,
            give=terms.pop('give', None), reply=[reply], sources=[source], **terms))

    offers.append(dict(npc=npc, keywords=['help'], topic=0, next_topic=9,
                       storage=[{**stage(0,0,0), 'preserve':True}], reply=[prompt]))
    reward('restart', 'help', 0, 9, [stage(1,2,0)], prompt)
    reward('first-donation', 'yes', 9, 10, [stage(0,0,1)],
           'Thank you. Could you spare another 500 gold for a tip?',
           fee=dict(gold=100, include_bank=True))
    reward('second-donation', 'yes', 10, 11, [stage(1,1,2)],
           'I have a key from Dermot. Would you buy it for 200 gold?',
           fee=dict(gold=500, include_bank=True))
    reward('key', 'yes', 11, 0, [{**stage(2,2,2), 'preserve':True}],
           'The dungeon key is yours.', fee=dict(gold=200, include_bank=True), repeatable=True,
           give=dict(item='oteryn:item.tibia.i2968', count=1, contents=[], key_number=3940))
