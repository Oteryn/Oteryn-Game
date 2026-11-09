"""M01b: explicit chosen EXPLORE anchors, executed by the existing STEP_IN owner."""
import json
import re
from pathlib import Path

AREA = 'content/world/areas/hunting-places/hunting-places-00000-00444.json'
BINDINGS = (
    ('oteryn:quest.horse_station_world_change', 's1', 'station-approach:palomino-pasture', 'chosen-source/horse_station_world_change'),
    ('oteryn:quest.hero_of_rathleton_quest', 's3', 'source-approach:hero-into-the-fire', 'chosen-source/hero_of_rathleton_quest'),
    ('oteryn:quest.grave_danger_quest', 's4', 'area-approach:oteryn:area.hunting_place.cobra_bastion', 'chosen-source/grave_danger_quest'),
    ('oteryn:quest.fury_gates_mini_world_change', 's1', 'fury-approach:thais', 'chosen-source/fury_gates_mini_world_change'),
    ('oteryn:quest.authored.fire_from_the_earth_mini_world_change', 's1', 'spawn-approach:oteryn:spawn.x32072_y32603_z2#0', 'authored/fire_from_the_earth_mini_world_change'),
    ('oteryn:quest.deeplings_worldchange', 's1', 'source-approach:deeplings-navigator-hub', 'chosen-source/deeplings_worldchange'),
    ('oteryn:quest.lion_trophy_quest', 's1', 'oteryn:area.hunting_place.formorgar_mines', 'chosen-source/lion_trophy_quest'),
    ('oteryn:quest.hero_of_rathleton_quest', 's1', 'npc-approach:oteryn:npc.strange_pipe', 'chosen-source/hero_of_rathleton_quest'),
    ('oteryn:quest.ghoul_room_quest', 's1', 'door-approach:canary:door-gate/key/3600', 'chosen-source/ghoul_room_quest'),
    ('oteryn:quest.bear_room_quest', 's1', 'door-approach:canary:door-gate/key/4601', 'chosen-source/bear_room_quest'),
    ('oteryn:quest.mad_mage_room_quest', 's1', 'npc-approach:oteryn:npc.a_prisoner', 'chosen-source/mad_mage_room_quest'),
    ('oteryn:quest.inukaya_quest', 's1', 'claim:canary:reward-claim/quest/u8_0/shardsof_ancient_winters/inukaya', 'chosen-source/inukaya_quest'),
    ('oteryn:quest.medusa_shield_quest', 's1', 'claim:canary:reward-claim/quest/u7_3/the_medusa_quest', 'chosen-source/medusa_shield_quest'),
    ('oteryn:quest.grimvale_mini_world_change', 's1', 'area-approach:oteryn:area.hunting_place.grimvale', 'chosen-source/grimvale_mini_world_change'),
    ('oteryn:quest.emperor_s_cookies_quest', 's1', 'claim:canary:reward-claim/quest/key/id3800', 'chosen-source/emperor_s_cookies_quest'),
    *(("oteryn:quest.grave_danger_quest", "s1", "source-approach:" + name,
       "chosen-source/grave_danger_quest") for name in (
           "grave-danger/dark-cathedral", "grave-danger/femor-hills", "grave-danger/ankrahmun",
           "grave-danger/vengoth", "grave-danger/orclands", "grave-danger/ice-islands",
           "grave-danger/kilmaresh", "grave-danger/duke-krule", "grave-danger/count-vlarkorth",
           "grave-danger/earl-osam", "grave-danger/lord-azaram", "grave-danger/baeloc-nictros")),
    ('oteryn:quest.ferumbras_ascension_quest', 's1', 'source-approach:ferumbras-entrance', 'chosen-source/ferumbras_ascension_quest'),
    ('oteryn:quest.heart_of_destruction_quest', 's1', 'source-approach:heart-zao-breach', 'chosen-source/heart_of_destruction_quest'),
    ('oteryn:quest.dragon_corpse_quest', 's1', 'area-approach:oteryn:area.hunting_place.rookgaard_bear_cave', 'chosen-source/dragon_corpse_quest'),
    ('oteryn:quest.fishing_box_quest', 's1', 'claim:canary:reward-claim/quest/u8_0/fishing_box', 'chosen-source/fishing_box_quest'),
    ('oteryn:quest.berserker_treasure_quest', 's1', 'claim:canary:reward-claim/quest/u6_4/berserker_treasure/rewards/white_pearls', 'chosen-source/berserker_treasure_quest'),
    ('oteryn:quest.iron_helmet_quest', 's1', 'claim:canary:reward-claim/kv/ironhelmet', 'chosen-source/iron_helmet_quest'),
    ('oteryn:quest.goblin_temple_quest', 's1', 'claim:canary:reward-claim/kv/goblintemple', 'chosen-source/goblin_temple_quest'),
    ('oteryn:quest.katana_quest', 's1', 'claim:canary:reward-claim/quest/key/id4603', 'chosen-source/katana_quest'),
    ('oteryn:quest.minotaur_hell_quest', 's1', 'claim:canary:reward-claim/kv/minohell1', 'chosen-source/minotaur_hell_quest'),
    ('oteryn:quest.frost_dragon_quest', 's1',
     'claim:canary:reward-claim/quest/u8_0/shardsof_ancient_winters/okolnir',
     'chosen-source/frost_dragon_quest'),
    ('oteryn:quest.behemoth_quest', 's1',
     'spawn-approach:oteryn:spawn.x33230_y31696_z12#3', 'chosen-source/behemoth_quest'),
    ('oteryn:quest.dragon_tower_quest', 's1',
     'area-approach:oteryn:area.hunting_place.shadowthorn', 'chosen-source/dragon_tower_quest'),
    ('oteryn:quest.berserk_potion_quest', 's1',
     'area-approach:oteryn:area.hunting_place.helheim', 'chosen-source/berserk_potion_quest'),
    ('oteryn:quest.fire_axe_quest', 's1',
     'area-approach:oteryn:area.hunting_place.edron_dragon_lair', 'chosen-source/fire_axe_quest'),
    *(("oteryn:quest.koshei_the_deathless_quest", "s1",
       "claim:canary:reward-claim/quest/u8_1/koshei_the_deathless/koshei_amulet_part" + str(part),
       "chosen-source/koshei_the_deathless_quest") for part in range(1, 5)),
    ('oteryn:quest.edron_goblin_quest', 's1',
     'oteryn:area.hunting_place.edron_goblin_cave', 'chosen-source/edron_goblin_quest'),
    ('oteryn:quest.lions_rock', 's1',
     'oteryn:area.hunting_place.lion_s_rock', 'chosen-source/lions_rock'),
    ('oteryn:quest.authored.elvenbane_quest', 's1',
     'oteryn:area.hunting_place.elvenbane', 'authored/elvenbane_quest'),
    ('oteryn:quest.authored.elvenbane_quest', 's2',
     'spawn-approach:oteryn:spawn.x32590_y31645_z5#0', 'authored/elvenbane_quest'),
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
    'oteryn:area.hunting_place.formorgar_mines': (32146, 31120, -9, 6685),
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
    import questpc_completed_quests
    import questpc_world_requirements
    world_owners = questpc_world_requirements.catalogue(root)
    areas = json.loads((Path(root) / AREA).read_text())['records']
    by_quest = {}
    for path in (Path(root) / 'content/quests/definitions').glob('quests-*.json'):
        for row in json.loads(path.read_text())['records']:
            definition = row['definition']
            by_quest.setdefault(definition['identity']['key'], []).append(definition)
    result = []
    for quest, stage, area, track_tail in BINDINGS:
        definitions = by_quest.get(quest, [])
        if len(definitions) != 1:
            raise ValueError('Explore binding needs one canonical quest: ' + quest)
        definition = definitions[0]
        recipe = definition.get('recipe') or definition.get('oteryn_recipe', {}).get('payload', {}).get('recipe')
        if not recipe:
            raise ValueError('Explore binding needs an explicit chosen recipe: ' + quest)
        named_visit = area.startswith('claim:') or (area.startswith('source-approach:') and '/' in area)
        visit = area.rsplit('/', 1)[-1] if named_visit else 'entry'
        storage = stage_storage(quest, stage, recipe, track_tail, visit)
        fury_approach = None
        station_approach = None
        if area.startswith('station-approach:'):
            import questpc_station_approaches
            station_approach = questpc_station_approaches.resolve(
                quest, stage, area, definition=definition)
            world_change = None
        elif area.startswith('fury-approach:'):
            import questpc_fury_approaches
            fury_approach = questpc_fury_approaches.resolve(root, quest, area)
            world_change = fury_approach[3]
        else:
            world_change = questpc_world_requirements.requirement(quest, world_owners)
        anchors = [a for q, s, a, _ in BINDINGS if (q, s) == (quest, stage)]
        # Every counted visit needs a distinct explicit physical anchor; a partial
        # fragment set cannot make the selected four-visit stage completable.
        if len(anchors) != storage[1]['maximum'] + 1 or len(set(anchors)) != len(anchors):
            raise ValueError('Explore binding needs an explicit complete visit anchor set: ' + quest)
        tail = quest.removeprefix('oteryn:quest.')
        requirements = recipe['requirements']
        completed_quests = questpc_completed_quests.resolve(quest, requirements['prerequisites'],
            [d for group in by_quest.values() for d in group])
        if station_approach is not None:
            at, appearance, sources = station_approach
        elif fury_approach is not None:
            at, appearance, sources = fury_approach[:3]
        elif area.startswith('door-approach:'):
            import questpc_door_approaches
            at, appearance, sources = questpc_door_approaches.resolve(root, quest, area, definition=definition)
        elif area.startswith('npc-approach:'):
            import questpc_npc_approaches
            at, appearance, sources = questpc_npc_approaches.resolve(root, quest, area)
        elif area.startswith('source-approach:'):
            import questpc_source_approaches
            at, appearance, sources = questpc_source_approaches.resolve(root, quest, area, definition=definition)
        elif area.startswith('spawn-approach:'):
            import questpc_spawn_approaches
            at, appearance, sources = questpc_spawn_approaches.resolve(root, quest, area)
        elif area.startswith('area-approach:'):
            import questpc_area_approaches
            at, appearance, sources = questpc_area_approaches.resolve(root, area)
        elif area.startswith('claim:'):
            import questpc_fragment_approaches
            at, appearance, sources = questpc_fragment_approaches.resolve(root, quest, area, definition=definition)
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
        if world_change is not None:
            result[-1]['world_change'] = world_change
        if requirements['premium']:
            result[-1]['admission'] = {'premium': True}
        if requirements['min_level']:
            result[-1]['minimum_level'] = requirements['min_level']
        if completed_quests:
            result[-1]['required_completed_quests'] = completed_quests
    return result
