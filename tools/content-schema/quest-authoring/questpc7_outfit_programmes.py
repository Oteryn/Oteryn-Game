"""Selected personal outfit programmes and explicit consumptive grant compatibility."""
import hashlib
import json

DECISION = 'owner:questpc7:selected-outfit-item-bindings:2026-11-07'
ITEM_ACTIONS = 'tools/content-schema/native-gameplay/item-cosmetic-actions.json'
SPECS = [
    ('fiend_slayer_outfits_quest', 'fiend', 'the_forgemaster', 'fiend_slayer',
     'no_rest_for_the_wicked_quest', (50060, 50061), (1, 2)),
    ('hand_of_the_inquisition_outfits_quest', 'inquisition', 'henricus', 'hand_of_the_inquisition',
     'grave_danger_quest', (31738, 31737), (1, 2)),
]


def consumptive_mappings(root, outfits, mappings):
    """Join declared look pairs, never grant-key spelling or display-name guesses."""
    raw = (root / ITEM_ACTIONS).read_bytes()
    document = json.loads(raw)
    wanted = {'oteryn:outfit.' + row[3] for row in SPECS} | {'oteryn:outfit.formal_dress'}
    selected = [row for row in outfits if row['key'] in wanted]
    if {row['key'] for row in selected} != wanted:
        raise ValueError('QUESTPC7_SELECTED_COSMETIC_MISSING')
    pairs = {tuple(sorted((row['female']['id'], row['male']['id']))): row for row in selected}
    if len(pairs) != len(selected):
        raise ValueError('QUESTPC7_AMBIGUOUS_COSMETIC_PAIR')
    for grant in document['grants']:
        row = pairs.get(tuple(sorted(grant.get('look_types', []))))
        if row is None or grant['kind'] not in ('outfit', 'addon'):
            continue
        if grant['kind'] == 'addon':
            if not 1 <= grant['addon'] <= row['addon_count']:
                raise ValueError('QUESTPC7_UNSUPPORTED_SOURCE_ADDON')
            target = {'kind': 'Addon', 'outfit_key': row['key'], 'addon': grant['addon']}
        else:
            target = {'kind': 'Outfit', 'outfit_key': row['key']}
        mapping = {'grant_key': grant['key'], 'target': target,
                   'compatible_provenance': [document['provenance']]}
        previous = next((row for row in mappings if row['grant_key'] == grant['key']), None)
        if previous is None:
            mappings.append(mapping)
        elif previous != mapping:
            raise ValueError('QUESTPC7_CONFLICTING_CONSUMPTIVE_GRANT')
    return {'reference': ITEM_ACTIONS, 'revision': document['crystal_commit'],
            'sha256': hashlib.sha256(raw).hexdigest()}


def append(root, callbacks, inputs, pin):
    definitions = {}
    for path in sorted((root / 'content/quests/definitions').glob('quests-*.json')):
        raw = path.read_bytes()
        for index, row in enumerate(json.loads(raw)['records']):
            definition = row['definition']
            definitions[definition['identity']['key']] = (path.relative_to(root).as_posix(), index, raw, definition)
    for suffix, slug, npc, outfit, parent_suffix, items, bits in SPECS:
        quest, (path, index, raw, definition) = next((key, value) for key, value in definitions.items() if key.endswith('.' + suffix))
        parent = next(key for key in definitions if key.endswith('.' + parent_suffix))
        recipe = definition.get('recipe') or definition['oteryn_recipe']['payload']['recipe']
        if ([s['kind'] for s in recipe['stages']] != ['talk', 'collect', 'use', 'collect', 'use', 'complete']
                or any(s['count'] != 1 for s in recipe['stages']) or recipe['repeat']['kind'] != 'once'):
            raise ValueError('QUESTPC7_SELECTED_OUTFIT_RECIPE_DRIFT:' + slug)
        source = f'data-global/npc/{npc}.lua'
        requirements = {'min_level': recipe['requirements']['min_level'],
                        'premium': recipe['requirements']['premium'], 'completed_quests': [parent]}
        sources = [f'canonical:{path}:1#/records/{index}/definition:sha256:{hashlib.sha256(raw).hexdigest()}',
                   f'crystal:{pin}:{source}:1-{len(inputs[source].splitlines())}:sha256:{hashlib.sha256(inputs[source]).hexdigest()}']
        track = f'oteryn:quest-progress/questpc7/{slug}/programme'

        def add(name, before, after, *, give=None, addon=None, owned=None, completes=False):
            req = dict(requirements)
            if owned is not None:
                req['owned_cosmetics'] = [{'outfit': 'oteryn:outfit.' + outfit, 'addons': owned}]
            callbacks.append({'key': f"oteryn:quest-reward/questpc7-{slug}-{name.replace(' ', '-')}", 'quest': quest,
                'npc': 'oteryn:npc.' + npc, 'keywords': [slug + ' ' + name], 'topic': 0, 'next_topic': 0,
                'storage': [{'track': track, 'minimum': before, 'maximum': before, 'set': after}],
                'requirements': req, 'take': [],
                'give': None if give is None else {'item': f'oteryn:item.tibia.i{give}', 'count': 1, 'contents': []},
                'outfit': None if addon is None else {'outfit': 'oteryn:outfit.' + outfit, 'addons': addon},
                'achievement': None, 'completes': completes,
                'reply': [f'Your {slug} {name} qualification is recorded.'], 'sources': sources})

        add('commission', -1, 0, addon=0)
        add('first token', 0, 1, give=items[0], owned=0)
        add('second token', 1, 2, give=items[1], owned=bits[0])
        add('complete', 2, 3, owned=3, completes=True)

def append_formal(root, callbacks, inputs, pin):
    """The chosen personal exhibition uses exact existing palace map objects."""
    path = 'content/quests/definitions/quests-00200-00299.json'
    raw = (root / path).read_bytes()
    index, definition = next((i, r['definition']) for i, r in enumerate(json.loads(raw)['records'])
                             if r['definition']['identity']['key'] == 'oteryn:quest.formal_dress_outfits_quest')
    recipe = definition['oteryn_recipe']['payload']['recipe']
    if ([s['kind'] for s in recipe['stages']] != ['explore', 'use', 'collect', 'use', 'collect', 'use', 'complete']
            or any(s['count'] != 1 for s in recipe['stages']) or recipe['repeat']['kind'] != 'once'):
        raise ValueError('QUESTPC7_FORMAL_SELECTED_RECIPE_DRIFT')
    quest = definition['identity']['key']
    track = 'oteryn:quest-progress/questpc7/formal/programme'
    outfit = 'oteryn:outfit.formal_dress'
    canonical = f'canonical:{path}:1#/records/{index}/definition:sha256:{hashlib.sha256(raw).hexdigest()}'
    requirements = {'min_level': recipe['requirements']['min_level'], 'premium': recipe['requirements']['premium'], 'completed_quests': []}
    npc_source = 'data-global/npc/king_tibianus.lua'
    npc_citation = f'crystal:{pin}:{npc_source}:1-{len(inputs[npc_source].splitlines())}:sha256:{hashlib.sha256(inputs[npc_source]).hexdigest()}'

    def state(before, after):
        return {'track': track, 'minimum': before, 'maximum': before, 'set': after}

    def add(name, before, after, words, reply, *, give=None, owned=None, completes=False):
        req = dict(requirements)
        if owned is not None:
            req['owned_cosmetics'] = [{'outfit': outfit, 'addons': owned}]
        callbacks.append({'key': f'oteryn:quest-reward/questpc7-formal-{name}', 'quest': quest,
            'npc': 'oteryn:npc.king_tibianus', 'keywords': words, 'topic': 0, 'next_topic': 0,
            'storage': [state(before, after)], 'requirements': req, 'take': [],
            'give': None if give is None else {'item': f'oteryn:item.tibia.i{give}', 'count': 1, 'contents': []},
            'outfit': None, 'achievement': 'oteryn:achievement/sharp_dressed' if completes else None,
            'completes': completes, 'reply': [reply], 'sources': [canonical, npc_citation]})

    add('admission', -1, 0, ['anniversary exhibition', 'formal commission'],
        'Visit our anniversary exhibition beside the northeastern palace display, then use the display to claim your Formal Dress.')
    add('button', 2, 3, ['formal brass button'],
        'Attach this Brass Button, then ask me for a {formal decorative plume}.', give=37604, owned=0)
    add('plume', 3, 4, ['formal decorative plume'],
        'Attach this Decorative Plume, then ask me to {complete formal dress}.', give=37605, owned=2)
    add('complete', 4, 5, ['complete formal dress'],
        'Your Formal Dress exhibition programme is complete. You are Sharp Dressed!', owned=3, completes=True)
    worlds = []
    for name, trigger, position, appearance, before, after, message, reward in [
        ('entry', 'step_in', {'x':32314,'y':32170,'floor':-6}, 464, 0, 1,
         'You have found the Formal Dress anniversary exhibition.', None),
        ('display', 'use', {'x':32315,'y':32170,'floor':-6}, 2110, 1, 2,
         'You have earned the Formal Dress outfit. Ask the king for a formal brass button.', {'outfit':outfit,'addons':0}),
    ]:
        row = {'key':f'oteryn:quest-callback/questpc7-formal-{name}', 'quest':quest,
            'trigger':trigger, 'selector':{'position':position,'unique':None,'action':None,'appearance':appearance},
            'lever':None,'reversible':False,'passages':[],'teleport':None,'storage':[state(before,after)],
            'minimum_level':20,'requirements':requirements,'experience':None,'monk_reward':None,
            'offerings':[],'item_on_target':None,'return_on_failure':False,
            'failure_message':'Ask the king about the anniversary exhibition first.',
            'success_message':message,'sources':[canonical]}
        if reward is not None:
            row['outfit_reward'] = reward
        worlds.append(row)
    return worlds
