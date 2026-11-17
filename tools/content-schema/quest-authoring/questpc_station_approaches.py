"""The persistent first station landmark is distinct from horse escape/raid state."""
QUEST = 'oteryn:quest.horse_station_world_change'
REFERENCE = 'station-approach:palomino-pasture'
SOURCE = ('https://raw.githubusercontent.com/zimbadev/crystalserver/'
          '201189f871d23597b82caf88fd611454a2ddefb8/'
          'data-global/scripts/globalevents/spawn/horse_station_world_change.lua')


def resolve(quest, stage, reference, *, definition):
    if (quest, stage, reference) != (QUEST, 's1', REFERENCE):
        raise ValueError('Not the reviewed permanent station survey')
    owner = definition.get('source_refs', {}).get('quest', {})
    if (definition['identity']['key'] != QUEST
            or owner.get('key') != 'canary:quest/horse_station_world_change'
            or owner.get('revision') != 'canary-04b83b51+crystalserver-9f5a72c6'):
        raise ValueError('Pinned station quest owner changed')
    recipe = (definition.get('recipe')
              or definition.get('oteryn_recipe', {}).get('payload', {}).get('recipe'))
    stages = recipe['stages'] if recipe else []
    if (not stages or stages[0]['key'] != 's1' or stages[0]['kind'] != 'explore'
            or stages[0]['count'] != 1 or stages[0]['objective'] !=
            'Visit the horse station east of Thais and inspect its pasture.'):
        raise ValueError('Permanent station survey contract changed')
    # Crystal's persistent startup pasture box:32438..32442,32231..32238,z7.
    # Inspect from two tiles north of its northeast corner; all fencing remains.
    return (dict(x=32442, y=32229, floor=-7), 4521,
            [SOURCE, 'OTERYN_CHOSEN_EXPLORE_ENTRY:source-map-approach;first-cycle-only',
             'OTERYN_PERMANENT_LANDMARK_SURVEY:horse-station/s1;no-escape-activation'])
