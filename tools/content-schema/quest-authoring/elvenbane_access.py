"""Chosen personal Elvenbane route; native geometry and donor treasure are evidence.

The recipe deliberately makes the Dragon and all five native upper defenders
mandatory. These phase callbacks are Oteryn choices, not donor storage claims.
"""
PHASE = 'oteryn:quest-progress/elvenbane/native-defenders/phase'
NAMES = ('dragon', 'elf-arcanist', 'demon-skeleton-east', 'demon-skeleton-west',
         'minotaur-mage', 'orc-shaman')


def callbacks(factory, position):
    sources = [
        'crystal:201189f871d23597b82caf88fd611454a2ddefb8:'
        'data-global/startup/tables/chest.lua:2553-2584'
        '#sha256:7c3ac1280196923e0bf5ca6f2ea932c19065bb9ac22d1559150a184f3cf06df4',
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
        if after == 1:
            row['storage'] += [dict(track='oteryn:quest-progress/elvenbane/native-defenders/' + name,
                                    minimum=1, maximum=1, set=1, preserve=True)
                               for name in NAMES]
        rows.append(row)
    return rows
