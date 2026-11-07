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
        mappings.append({'grant_key': grant['key'], 'target': target,
                         'compatible_provenance': [document['provenance']]})
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
