"""The exact retained Dead Explorer selector and one-time scale reward."""


def dead_explorer(callback, position):
    script = 'data-global/scripts/quests/the_great_dragon_hunt_quest/actions_warrior_skeleton.lua'
    row = callback('great-dragon-hunt-dead-explorer', 'adventurers_guild', 'use',
                   script + ':3-16,21', at=position(33007, 32607, 8), appearance=21412)
    row.update(
        storage=[{
            'track': 'oteryn:quest-progress/quest/u10_80/the_great_dragon_hunt/warrior_skeleton',
            'minimum': 0, 'maximum': 0, 'set': 1,
        }],
        requires_premium=True,
        item_reward={'item': 'oteryn:item.tibia.i5882', 'count': 1, 'contents': []},
        failure_message='The dead explorer is empty.',
        success_message="You have discovered a deceased warrior's skeleton. It seems he tried to hunt the dragons around here - and failed.",
    )
    row['sources'].extend([
        'crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/item.lua:3390-3394',
        'https://tibia.fandom.com/?curid=75167&oldid=847146',
        'https://tibia.fandom.com/?curid=75348&oldid=1211828',
    ])
    return row
