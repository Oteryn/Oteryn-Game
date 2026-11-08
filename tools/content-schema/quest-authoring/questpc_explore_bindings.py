"""M01b: explicit chosen EXPLORE anchors, executed by the existing STEP_IN owner."""
import json
from pathlib import Path

AREA = 'content/world/areas/hunting-places/hunting-places-00000-00444.json'
BINDINGS = (
    ('oteryn:quest.longsword_quest', 's1',
     'oteryn:area.hunting_place.dusalk_s_troll_clan_cave', 'chosen-source/longsword_quest'),
    ('oteryn:quest.geomancer_quest', 's1',
     'oteryn:area.hunting_place.mount_sternum_undead_cave', 'chosen-source/geomancer_quest'),
)
ANCHORS = {
    'oteryn:area.hunting_place.dusalk_s_troll_clan_cave': (32663, 31970, -7, 4525),
    'oteryn:area.hunting_place.mount_sternum_undead_cave': (32418, 32017, -7, 4529),
}


def callbacks(root):
    areas = json.loads((Path(root) / AREA).read_text())['records']
    result = []
    for quest, stage, area, track_tail in BINDINGS:
        definitions = [r['definition']
                       for path in (Path(root) / 'content/quests/definitions').glob('quests-*.json')
                       for r in json.loads(path.read_text())['records']
                       if r['definition']['identity']['key'] == quest]
        if len(definitions) != 1:
            raise ValueError('Explore binding needs one canonical quest: ' + quest)
        definition = definitions[0]
        recipe = definition.get('recipe') or definition.get('oteryn_recipe', {}).get('payload', {}).get('recipe')
        if not recipe:
            raise ValueError('Explore binding needs an explicit chosen recipe: ' + quest)
        stages = [s for s in recipe['stages'] if s['key'] == stage and s['kind'] == 'explore']
        if len(stages) != 1:
            raise ValueError('Explore binding needs one EXPLORE stage: ' + quest + '/' + stage)
        # Additional visits and preceding objectives need their own explicit guards.
        if stages[0]['count'] != 1 or recipe['stages'][0]['key'] != stage:
            raise ValueError('Explore binding needs explicit visit and predecessor guards: ' + quest)
        tail = quest.removeprefix('oteryn:quest.')
        expected_tail = ('authored/' + tail.removeprefix('authored.')
                         if tail.startswith('authored.') else 'chosen-source/' + tail)
        if track_tail != expected_tail:
            raise ValueError('Explore counter owner differs from canonical quest: ' + quest)
        requirements = recipe['requirements']
        if (requirements['premium'] or requirements['min_level'] != 0
                or requirements['prerequisites']):
            raise ValueError('Explore route requires additional authoritative guards: ' + quest)
        matches = [(i, r) for i, r in enumerate(areas)
                   if r['declaration']['identity']['key'] == area]
        if len(matches) != 1:
            raise ValueError('Explore anchor needs one canonical Area: ' + area)
        index, record = matches[0]
        source = record['declaration']['position']
        at = dict(x=source['x'], y=source['y'], floor=-source['floor'])
        # This is the pinned Area's declared entry point, not a guessed room box.
        x, y, floor, appearance = ANCHORS[area]
        if (at['x'], at['y'], at['floor']) != (x, y, floor):
            raise ValueError('Canonical entry point changed: ' + area)
        source_bindings = [b for b in record['source_bindings']
                           if b['identity_namespace'] == 'tibiawiki-fandom/page-id'
                           and b['disposition'] == 'EXACT']
        if len(source_bindings) != 1:
            raise ValueError('Explore anchor needs one exact pinned source: ' + area)
        binding = source_bindings[0]
        url = ('https://tibia.fandom.com/index.php?curid=' + binding['external_id']
               + '&oldid=' + binding['source_revision'])
        slug = tail.removeprefix('authored.').removesuffix('_quest').replace('_', '-')
        track = 'oteryn:quest-progress/' + track_tail + '/' + stage
        result.append(dict(
            key='oteryn:quest-callback/m01b/' + slug + '/' + stage + '/entry', quest=quest,
            trigger='step_in', selector=dict(position=at, unique=None, action=None,
                appearance=appearance, native=dict(position=at, ordinal=0, appearance=appearance)),
            lever=None, reversible=False, passages=[], teleport=None,
            storage=[dict(track=track + '/visited/entry', minimum=0, maximum=0, set=1),
                     dict(track=track, minimum=0, maximum=0, set=1, increment=True)],
            offerings=[], return_on_failure=False, failure_message=None,
            requires_world_admission=True,
            sources=[AREA + '#/records/' + str(index),
                url,
                'OTERYN_CHOSEN_EXPLORE_ENTRY:canonical-area-point;first-cycle-only']))
    return result
