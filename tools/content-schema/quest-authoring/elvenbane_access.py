"""Chosen personal Elvenbane route; native geometry and donor treasure are evidence.

The recipe deliberately makes the Dragon and all five native upper defenders
mandatory. These phase callbacks are Oteryn choices, not donor storage claims.
"""
PHASE = 'oteryn:quest-progress/elvenbane/native-defenders/phase'
NAMES = ('dragon', 'elf-arcanist', 'demon-skeleton-east', 'demon-skeleton-west',
         'minotaur-mage', 'orc-shaman')


def callbacks(factory, position):
    sources = [
        'crystal:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:'
        'data-global/startup/tables/chest.lua:2551-2580'
        '#sha256:0ca1a7ad6da9589d1b77ff847a70f306b2086464e7dce21afad53e0c852478ca',
        'owner:questpc2:authored68-elvenbane-personal-defender-route:2026-11-07',
        'https://tibia.fandom.com/index.php?curid=8007&oldid=265159',
    ]
    rows = []
    for key, at, before, after in [
        ('elvenbane-basement-entry', (32598, 31641, 8), -1, 0),
        ('elvenbane-return-after-defenders', (32597, 31642, 7), 0, 1),
    ]:
        row = factory(key, 'authored.elvenbane_quest', 'step_in', '', at=position(*at))
        row['sources'] = sources
        row['storage'] = [dict(track=PHASE, minimum=before, maximum=before, set=after)]
        if after == 0:
            # Both physical basement approaches share the closed once-only activation.
            row['selector']['position_aliases'] = [position(32598, 31653, 8)]
            row['storage'].append(dict(track='oteryn:quest-progress/elvenbane/phase',
                                       minimum=0, maximum=0, set=1))
        if after == 1:
            row['storage'] += [dict(track='oteryn:quest-progress/elvenbane/native-defenders/' + name,
                                    minimum=1, maximum=1, set=1, preserve=True)
                               for name in NAMES]
            row['storage'].append(dict(track='oteryn:quest-progress/elvenbane/phase',
                                       minimum=3, maximum=3, set=4))
            row['effects']['quest_completion'] = dict(claimed_rewards=[
                'oteryn:reward-claim.quest.u8_1.to_blind_the_enemy.' + name
                for name in ('bp1', 'bp2', 'dwarven_shield', 'morning_star')])
        rows.append(row)
    return rows
