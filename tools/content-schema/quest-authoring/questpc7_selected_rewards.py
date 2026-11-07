"""Native compositions of the lane's pinned, explicitly chosen quest recipes."""
import hashlib
import json


def append_golden(root, callbacks, offers, inputs, pin):
    path = 'content/quests/definitions/quests-00100-00199.json'
    raw = (root / path).read_bytes()
    records = json.loads(raw)['records']
    index, definition = next((i, r['definition']) for i, r in enumerate(records)
                             if r['definition']['identity']['key'] == 'oteryn:quest.golden_outfits_quest')
    recipe = definition['oteryn_recipe']['payload']['recipe']
    prices = [s['count'] for s in recipe['stages'] if s['kind'] == 'use']
    if prices != [100000, 50000, 50000] or recipe['repeat']['kind'] != 'once':
        raise ValueError('GOLDEN_SELECTED_RECIPE_DRIFT')
    source = 'data-global/npc/king_tibianus.lua'
    sources = [f'canonical:{path}:1#/records/{index}/definition:sha256:{hashlib.sha256(raw).hexdigest()}',
               f'crystal:{pin}:{source}:131-189:sha256:{hashlib.sha256(inputs[source]).hexdigest()}']
    track = 'oteryn:quest-progress/questpc7/golden/commission'
    npc = 'oteryn:npc.king_tibianus'

    def storage(before, after, preserve=False):
        return {'track': track, 'minimum': before, 'maximum': before, 'set': after,
                **({'preserve': True} if preserve else {})}

    def add(name, before, after, words, reply, *, topic=0, money=None, addon=None, completes=False):
        row = {'key': 'oteryn:quest-reward/questpc7-golden-' + name,
               'quest': definition['identity']['key'], 'npc': npc, 'keywords': words,
               'topic': topic, 'next_topic': 0, 'storage': [storage(before, after)],
               'take': [], 'give': None,
               'outfit': None if addon is None else {'outfit': 'oteryn:outfit.golden', 'addons': addon},
               'achievement': None, 'reply': [reply], 'sources': sources}
        if money is not None:
            row['fee'] = {'gold': money, 'include_bank': True}
        if completes:
            row['completes'] = True
        callbacks.append(row)

    add('intro', -1, 0, ['golden commission', 'golden outfit'],
        'I can commission the Golden outfit and its two addons. Ask me for the base outfit.')
    for stage, (name, words, label, price, addon) in enumerate([
        ('base', ['base outfit'], 'basic Golden outfit', prices[0], 0),
        ('gloves', ['boots and gloves'], 'Golden boots and gloves', prices[1], 1),
        ('helmet', ['golden helmet'], 'Golden helmet', prices[2], 2),
    ]):
        topic = stage + 1
        offers.append({'npc': npc, 'keywords': words, 'topic': 0, 'next_topic': topic,
                       'storage': [storage(stage, stage, True)],
                       'reply': [f'The {label} costs {price} gold. Do you agree?']})
        add(name, stage, stage + 1, ['yes'], f'Your {label} is ready.',
            topic=topic, money=price, addon=addon)
    add('complete', 3, 4, ['complete commission'], 'Your Golden commission is complete.', completes=True)
