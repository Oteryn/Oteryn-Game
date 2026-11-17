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
        # Keep the selected commission separate from main's donor topics1-5.
        topic = 101 + stage
        offers.append({'npc': npc, 'keywords': words, 'topic': 0, 'next_topic': topic,
                       'storage': [storage(stage, stage, True)],
                       'reply': [f'The {label} costs {price} gold. Do you agree?']})
        add(name, stage, stage + 1, ['yes'], f'Your {label} is ready.',
            topic=topic, money=price, addon=addon)
    add('complete', 3, 4, ['complete commission'], 'Your Golden commission is complete.', completes=True)


def append_friends(root, callbacks, offers, inputs, pin):
    """The explicitly selected finite programme, plus its two unlocked services."""
    path = 'content/quests/definitions/quests-00100-00199.json'
    raw = (root / path).read_bytes()
    index, definition = next((i, r['definition']) for i, r in enumerate(json.loads(raw)['records'])
                             if r['definition']['identity']['key'] == 'oteryn:quest.friends_and_traders')
    recipe = definition['oteryn_recipe']['payload']['recipe']
    priests = ['quentin', 'tibra', 'maealil', 'yberius', 'isimov', 'amanda', 'kasmir', 'rahkem', 'brewster', 'tyrias']
    prayer_targets = [s['targets'][0].lower() for s in recipe['stages'] if s['kind'] == 'talk' and s['key'] != 's1']
    if prayer_targets != priests or recipe['repeat']['kind'] != 'once':
        raise ValueError('FRIENDS_SELECTED_RECIPE_DRIFT')
    track = 'oteryn:quest-progress/questpc7/friends/program'
    canonical = f'canonical:{path}:1#/records/{index}/definition:sha256:{hashlib.sha256(raw).hexdigest()}'

    def add(name, npc, before, after, words, reply, *, take=(), give=None, present=(), fee=None,
            completes=False, counter=None, maximum=None):
        source = f'data-global/npc/{npc}.lua'
        source_bytes = inputs[source]
        state = {'track': track, 'minimum': before, 'maximum': before if maximum is None else maximum,
                 'set': after, **({'preserve': True} if counter else {})}
        row = {'key': 'oteryn:quest-reward/questpc7-friends-' + name,
               'quest': definition['identity']['key'], 'npc': 'oteryn:npc.' + npc,
               'keywords': words, 'topic': 0, 'next_topic': 0, 'storage': [state],
               'take': [{'item': f'oteryn:item.tibia.i{i}', 'count': n} for i, n in take],
               'present': [{'item': f'oteryn:item.tibia.i{i}', 'count': n} for i, n in present],
               'give': None if give is None else {'item': f'oteryn:item.tibia.i{give[0]}', 'count': give[1], 'contents': []},
               'outfit': None, 'achievement': None, 'reply': [reply],
               'sources': [canonical, f'crystal:{pin}:{source}:1-{len(source_bytes.splitlines())}:sha256:{hashlib.sha256(source_bytes).hexdigest()}']}
        if fee: row['fee'] = fee
        if completes: row['completes'] = True
        if counter: row['counter'] = {'track': 'oteryn:quest-progress/questpc7/friends/' + counter, 'maximum': 1000000000000}
        callbacks.append(row)

    add('intro', 'a_sweaty_cyclops', -1, 0, ['forging friendship'], 'Bring three bast skirts for my partner.')
    add('skirts', 'a_sweaty_cyclops', 0, 1, ['deliver skirts'], 'Now I can forge royal steel for you.', take=[(3560, 3)])
    add('honey', 'marina', 1, 2, ['deliver honey'], 'I can now spin ten spider silks into one spool of yarn.', take=[(5902, 50)])
    # The selected recipe specifies acquisition from Gamon without replacing his source price.
    add('stake', 'gamon', 2, 3, ['buy wooden stake'], 'Here is your wooden stake.', give=(5941, 1), fee={'gold': 5000, 'include_bank': True})
    for number, npc in enumerate(priests):
        add('prayer-' + npc, npc, 3 + number, 4 + number, ['stake prayer'],
            'Your stake has received this line of the prayer.', present=[(5941, 1)])
        # An out-of-order prayer is a normal reply, with no quest transition.
        # Eligible reward callbacks are selected before these ordinary offers.
        offers.append({'npc':'oteryn:npc.'+npc,'keywords':['stake prayer'],
                       'topic':0,'next_topic':0,
                       'storage':[{'track':track,'minimum':-1,'maximum':15,'set':-1,'preserve':True}],
                       'reply':['Follow the selected prayer order and keep your wooden stake.']})
    add('blessing', 'quentin', 13, 14, ['finish prayer'], 'The ten-part prayer has blessed your wooden stake.',
        take=[(5941, 1)], give=(5942, 1))
    offers.append({'npc':'oteryn:npc.quentin','keywords':['finish prayer'],
                   'topic':0,'next_topic':0,
                   'storage':[{'track':track,'minimum':-1,'maximum':15,'set':-1,'preserve':True}],
                   'reply':['The prayer may bless your wooden stake only once.']})
    add('complete', 'quentin', 14, 15, ['complete friendships'], 'Your Friends and Traders programme is complete.', completes=True)
    add('royal-steel', 'a_sweaty_cyclops', 1, 1, ['forge royal steel'], 'Cling clang!',
        take=[(3381, 1)], give=(5887, 1), counter='steel-uses', maximum=15)
    add('yarn', 'marina', 2, 2, ['spin yarn'], 'Here is your spool of yarn.',
        take=[(5879, 10)], give=(5886, 1), counter='yarn-uses', maximum=15)


def append_eligibility_starts(root, callbacks, inputs, pin):
    """Own quest starts; subsequent native routes follow the shared eligibility owner."""
    wanted = [('oteryn:quest.hunter_outfits_quest', 'hunter', 'elane', 'hunter commission'),
              ('oteryn:quest.authored.illuminated_warrior_outfits_quest', 'illuminated', 'wayland_smythers', 'illuminated commission')]
    definitions = {}
    for path in sorted((root / 'content/quests/definitions').glob('quests-*.json')):
        raw = path.read_bytes()
        for index, row in enumerate(json.loads(raw)['records']):
            definition = row['definition']
            if definition['identity']['key'] in {r[0] for r in wanted}:
                definitions[definition['identity']['key']] = (path.relative_to(root).as_posix(), index, raw, definition)
    for quest, slug, npc, words in wanted:
        path, index, raw, definition = definitions[quest]
        recipe = definition.get('recipe') or definition['oteryn_recipe']['payload']['recipe']
        requirements = recipe['requirements']
        source_path = f'data-global/npc/{npc}.lua'
        source_pin = ('crystal-summer:00ce02a57ca5a12e48f32a3476e37471167e4c3f'
                      if npc == 'wayland_smythers' else f'crystal:{pin}')
        callbacks.append({'key': f'oteryn:quest-reward/questpc7-{slug}-intro', 'quest': quest,
            'npc': 'oteryn:npc.' + npc, 'keywords': [words], 'topic': 0, 'next_topic': 0,
            'storage': [{'track': f'oteryn:quest-progress/questpc7/{slug}/programme', 'minimum': -1, 'maximum': -1, 'set': 0}],
            'requirements': {'min_level': requirements['min_level'], 'premium': requirements['premium'],
                'completed_quests': ['oteryn:quest.authored.make_believe_quest'] if slug == 'illuminated' else []},
            'take': [], 'give': None, 'outfit': None, 'achievement': None,
            'reply': ['Bring back my crossbow from the cultist cache.' if slug == 'hunter' else 'Collect 2500 radiant attire scraps for the basic outfit.'],
            'sources': [f'canonical:{path}:1#/records/{index}/definition:sha256:{hashlib.sha256(raw).hexdigest()}',
                f'{source_pin}:{source_path}:1-{len(inputs[source_path].splitlines())}:sha256:{hashlib.sha256(inputs[source_path]).hexdigest()}']})


def append_tokens(root, callbacks, inputs, pin):
    import hashlib
    import json
    for path in sorted((root/'content/quests/definitions').glob('quests-*.json')):
        raw=path.read_bytes()
        for index,row in enumerate(json.loads(raw)['records']):
            definition=row['definition']
            if definition['identity']['key']!='oteryn:quest.hunting_for_tokens':continue
            recipe=definition.get('recipe') or definition['oteryn_recipe']['payload']['recipe']
            if [(s['kind'],s['count']) for s in recipe['stages']] != [('talk',1),('collect',20),('use',20),('complete',1)] or recipe['repeat']['kind']!='once' or recipe['requirements']['prerequisites']:
                raise ValueError('QUESTPC7_TOKEN_SELECTED_RECIPE_DRIFT')
            source='data-global/npc/jorge.lua'
            if b'name = "Abacus", id = 19151' not in inputs[source] or b'doPlayerRemoveItem(creature, 19083, 20)' not in inputs[source]:
                raise ValueError('QUESTPC7_JORGE_EXACT_TOKEN_SOURCE_DRIFT')
            sources=[f'canonical:{path.relative_to(root).as_posix()}:1#/records/{index}/definition:sha256:{hashlib.sha256(raw).hexdigest()}',
                     f'crystal:{pin}:{source}:61,115,125-128:sha256:{hashlib.sha256(inputs[source]).hexdigest()}']
            for name,before,after,take,give,completes,reply in [
                ('replica rewards',-1,0,[],None,False,'Bring twenty Silver Raid Tokens and ask for an {abacus replica}.'),
                ('abacus replica',0,1,[{'item':'oteryn:item.tibia.i19083','count':20}],{'item':'oteryn:item.tibia.i19151','count':1,'contents':[]},False,'Your twenty Silver Raid Tokens paid for this Abacus replica. Ask to {finish the commission}.'),
                ('finish the commission',1,2,[],None,True,'Your replica commission is complete. Enjoy your Abacus!'),
            ]:
                callbacks.append({'key':'oteryn:quest-reward/questpc7-tokens-'+name.replace(' ','-'),
                    'quest':definition['identity']['key'],'npc':'oteryn:npc.jorge','keywords':[name],'topic':0,'next_topic':0,
                    'storage':[{'track':'oteryn:quest-progress/questpc7/tokens/programme','minimum':before,'maximum':before,'set':after}],
                    'requirements':{'min_level':recipe['requirements']['min_level'],'premium':recipe['requirements']['premium'],'completed_quests':[]},'take':take,'give':give,
                    'outfit':None,'achievement':None,'completes':completes,'reply':[reply],'sources':sources})
            return
    raise ValueError('QUESTPC7_TOKEN_CANONICAL_MISSING')
