"""Chosen live entrance guards; source anchors are verified against the retained map."""


def callbacks():
    source = 'crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/'
    entrances = []
    # Source registerArea covers all nine retained pieces of the Thais machine.
    for y in range(32164, 32167):
        for x in range(32429, 32432):
            appearance = 25091 + (y - 32164) * 3 + x - 32429
            name = 'anniversary' if appearance == 25096 else f'anniversary-{appearance}'
            entrances.append((
                name, '25_years_of_tibia_quest', 'use', (x, y, -8), 1, appearance,
                20, dict(regular_blessings=True, promotion=True, premium=True),
                (33453, 31030, -8), '25_years_of _tibia_quest/actions_time_machine.lua:1-49'))
    entrances.append((
        'pits', 'the_pits_of_inferno_quest', 'step_in', (32791, 32331, -10), 0, 354,
        80, dict(premium=True), None,
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
