"""M01b: explicit chosen EXPLORE anchors, executed by the existing STEP_IN owner."""
import json
import re
from pathlib import Path

AREA = 'content/world/areas/hunting-places/hunting-places-00000-00444.json'
BINDINGS = (
    ('oteryn:quest.edron_goblin_quest', 's1',
     'oteryn:area.hunting_place.edron_goblin_cave', 'chosen-source/edron_goblin_quest'),
    ('oteryn:quest.lions_rock', 's1',
     'oteryn:area.hunting_place.lion_s_rock', 'chosen-source/lions_rock'),
    ('oteryn:quest.authored.elvenbane_quest', 's1',
     'oteryn:area.hunting_place.elvenbane', 'authored/elvenbane_quest'),
    ('oteryn:quest.alawar_s_vault_quest', 's1',
     'callback:oteryn:quest-callback/alawars-vault-entrance', 'chosen-source/alawar_s_vault_quest'),
    ('oteryn:quest.longsword_quest', 's1',
     'oteryn:area.hunting_place.dusalk_s_troll_clan_cave', 'chosen-source/longsword_quest'),
    ('oteryn:quest.authored.asura_palace_quest', 's1',
     'oteryn:area.hunting_place.asura_palace', 'authored/asura_palace_quest'),
    ('oteryn:quest.geomancer_quest', 's1',
     'oteryn:area.hunting_place.mount_sternum_undead_cave', 'chosen-source/geomancer_quest'),
)
ANCHORS = {
    'oteryn:area.hunting_place.edron_goblin_cave': (33108, 31850, -7, 355),
    'oteryn:area.hunting_place.lion_s_rock': (33138, 32358, -7, 4399),
    'oteryn:area.hunting_place.elvenbane': (32590, 31645, -7, 436),
    'oteryn:area.hunting_place.asura_palace': (32953, 32685, -7, 923),
    'oteryn:area.hunting_place.edron_dragon_lair': (33098, 31701, -7, 4409),
    'oteryn:area.hunting_place.dusalk_s_troll_clan_cave': (32663, 31970, -7, 4525),
    'oteryn:area.hunting_place.mount_sternum_undead_cave': (32418, 32017, -7, 4529),
}



def stage_storage(quest, stage, recipe, track_tail, visit='entry'):
    """A visit is once-only; each preceding selected stage must already be complete."""
    matches = [(index, row) for index, row in enumerate(recipe['stages'])
               if row['key'] == stage and row['kind'] == 'explore']
    if len(matches) != 1:
        raise ValueError('Explore binding needs one EXPLORE stage: ' + quest + '/' + stage)
    index, bound = matches[0]
    tail = quest.removeprefix('oteryn:quest.')
    expected = ('authored/' + tail.removeprefix('authored.')
                if tail.startswith('authored.') else 'chosen-source/' + tail)
    if track_tail != expected:
        raise ValueError('Explore counter owner differs from canonical quest: ' + quest)
    if not re.fullmatch(r'[a-z0-9_-]{1,64}', visit):
        raise ValueError('Explore visit needs a bounded stable identifier')
    count = bound['count']
    if not isinstance(count, int) or isinstance(count, bool) or count < 1:
        raise ValueError('Explore stage needs a positive integral visit count')
    prefix = 'oteryn:quest-progress/' + track_tail + '/'
    track = prefix + stage
    result = [dict(track=track + '/visited/' + visit, minimum=0, maximum=0, set=1),
              dict(track=track, minimum=0, maximum=count - 1, set=1, increment=True)]
    # The chosen programme selects an ordered finite sequence. Read guards never
    # manufacture predecessor credit or reserve mutable storage for another stage.
    for previous in recipe['stages'][:index]:
        result.append(dict(track=prefix + previous['key'], minimum=previous['count'],
                           maximum=previous['count'], set=previous['count'], preserve=True))
    return result


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
        visit = area.rsplit('/', 1)[-1] if area.startswith('claim:') else 'entry'
        storage = stage_storage(quest, stage, recipe, track_tail, visit)
        anchors = [a for q, s, a, _ in BINDINGS if (q, s) == (quest, stage)]
        # Every counted visit needs a distinct explicit physical anchor; a partial
        # fragment set cannot make the selected four-visit stage completable.
        if len(anchors) != storage[1]['maximum'] + 1 or len(set(anchors)) != len(anchors):
            raise ValueError('Explore binding needs an explicit complete visit anchor set: ' + quest)
        tail = quest.removeprefix('oteryn:quest.')
        requirements = recipe['requirements']
        if requirements['prerequisites']:
            raise ValueError('Explore route requires additional authoritative guards: ' + quest)
        if area.startswith('claim:'):
            import questpc_fragment_approaches
            at, appearance, sources = questpc_fragment_approaches.resolve(root, quest, area)
        elif area.startswith('callback:'):
            import questpc_map_approaches
            at, appearance, sources = questpc_map_approaches.resolve(root, quest, area)
        else:
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
            sources = [AREA + '#/records/' + str(index), url,
                       'OTERYN_CHOSEN_EXPLORE_ENTRY:canonical-area-point;first-cycle-only']
        slug = tail.removeprefix('authored.').removesuffix('_quest').replace('_', '-')
        result.append(dict(
            key='oteryn:quest-callback/m01b/' + slug + '/' + stage + '/' + visit, quest=quest,
            trigger='step_in', selector=dict(position=at, unique=None, action=None,
                appearance=appearance, native=dict(position=at, ordinal=0, appearance=appearance)),
            lever=None, reversible=False, passages=[], teleport=None,
            storage=storage,
            offerings=[], return_on_failure=False, failure_message=None,
            requires_world_admission=True,
            sources=sources))
        if requirements['premium']:
            result[-1]['admission'] = {'premium': True}
        if requirements['min_level']:
            result[-1]['minimum_level'] = requirements['min_level']
    return result
