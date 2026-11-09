"""Chosen live entrance guards; source anchors are verified against the retained map."""

import json
from pathlib import Path


def _requirements(root, quest, shard, route_tools=()):
    path = Path(root) / 'content/quests/definitions' / shard
    definitions = [record['definition'] for record in json.loads(path.read_text())['records']
                   if record['definition']['identity']['key'] == 'oteryn:quest.' + quest]
    if len(definitions) != 1:
        raise ValueError('M03 needs one canonical quest definition: ' + quest)
    definition = definitions[0]
    recipe = definition.get('recipe') or definition.get('oteryn_recipe', {}).get('payload', {}).get('recipe')
    if not recipe:
        raise ValueError('M03 needs canonical recipe requirements: ' + quest)
    requirements = recipe['requirements']
    level, premium, prerequisites = (requirements['min_level'], requirements['premium'],
                                      requirements['prerequisites'])
    if (set(requirements) - {'basis', 'min_level', 'premium', 'prerequisites'}
            or type(level) is not int or not 0 <= level <= 2**32 - 1
            or type(premium) is not bool or not isinstance(prerequisites, list)
            or any(not isinstance(value, str) for value in prerequisites)):
        raise ValueError('invalid canonical admission requirements: ' + quest)
    prerequisites = set(prerequisites)
    unsupported = prerequisites - {'All regular blessings', 'Vocation promotion'} - set(route_tools)
    if unsupported:
        raise ValueError('unsupported prerequisite for ' + quest + ': ' + repr(sorted(unsupported)))
    admission = {}
    if 'All regular blessings' in prerequisites:
        admission['regular_blessings'] = True
    if 'Vocation promotion' in prerequisites:
        admission['promotion'] = True
    if premium:
        admission['premium'] = True
    return level or None, admission or None


def callbacks(root):
    source = 'crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/'
    anniversary_level, anniversary_admission = _requirements(
        root, '25_years_of_tibia_quest', 'quests-00200-00299.json')
    pits_level, pits_admission = _requirements(
        root, 'the_pits_of_inferno_quest', 'quests-00000-00099.json', ('Rope', 'Shovel'))
    entrances = []
    # Source registerArea covers all nine retained pieces of the Thais machine.
    for y in range(32164, 32167):
        for x in range(32429, 32432):
            appearance = 25091 + (y - 32164) * 3 + x - 32429
            name = 'anniversary' if appearance == 25096 else f'anniversary-{appearance}'
            entrances.append((
                name, '25_years_of_tibia_quest', 'use', (x, y, -8), 1, appearance,
                anniversary_level, dict(anniversary_admission) if anniversary_admission else None,
                (33453, 31030, -8), '25_years_of _tibia_quest/actions_time_machine.lua:1-49'))
    entrances.append((
        'pits', 'the_pits_of_inferno_quest', 'step_in', (32791, 32331, -10), 0, 354,
        pits_level, pits_admission, None,
        'the_pits_of_inferno_quest/movements_holy_tible_tile.lua:1-4'))
    rows = []
    for name, quest, trigger, at, ordinal, appearance, level, admission, destination, citation in entrances:
        position = dict(zip(('x', 'y', 'floor'), at))
        rows.append(dict(
            key='oteryn:quest-callback/qm03/' + name, quest='oteryn:quest.' + quest,
            trigger=trigger, selector=dict(position=position, unique=None, action=None,
                appearance=appearance, native=dict(position=position, ordinal=ordinal,
                                                    appearance=appearance)),
            admission=admission, minimum_level=level,
            lever=None, reversible=False, passages=[],
            teleport=dict(zip(('x', 'y', 'floor'), destination)) if destination else None,
            storage=[dict(track='oteryn:quest-progress/chosen-source/' + quest + '/s1',
                          minimum=-1, maximum=1, set=1, at_least=True)],
            requires_world_admission=True, offerings=[],
            return_on_failure=trigger == 'step_in',
            failure_message='You do not meet the entrance requirements.',
            sources=[source + citation,
                     'OTERYN_CHOSEN_ADMISSION:recipe.requirements;initial-entry-only',
                     'OTERYN_CHOSEN_ENTRANCE:retained-source-destination' if name == 'pits'
                     else 'OTERYN_SOURCE_USE:retained-time-machine-counterpart']))
    return rows
