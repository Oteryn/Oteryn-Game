"""M01b keeps native entry selectors and separate once-only visit counters exact."""
import unittest
import json
import sys
from unittest import mock
from pathlib import Path

import questpc_explore_bindings as explore

ROOT = Path(__file__).resolve().parents[3]


def canonical_recipe(quest):
    for path in (ROOT / 'content/quests/definitions').glob('quests-*.json'):
        for row in json.loads(path.read_text())['records']:
            definition = row['definition']
            if definition['identity']['key'] == quest:
                return (definition.get('recipe')
                        or definition['oteryn_recipe']['payload']['recipe'])
    raise AssertionError('Missing canonical quest: ' + quest)


class ExploreBindingsTests(unittest.TestCase):
    def test_volcano_spawn_distance_exception_stays_bounded(self):
        import questpc_spawn_approaches as spawns
        key = 'oteryn:spawn.x32072_y32603_z2#0'
        with mock.patch.dict(spawns.DISTANCES, {key:3}):
            with self.assertRaisesRegex(ValueError,'declared distance'):
                spawns.resolve(ROOT,'oteryn:quest.authored.fire_from_the_earth_mini_world_change',
                               'spawn-approach:' + key)

    def test_fire_earth_survey_uses_the_qualified_volcano_spawn_approach(self):
        bindings = (('oteryn:quest.authored.fire_from_the_earth_mini_world_change','s1',
                     'spawn-approach:oteryn:spawn.x32072_y32603_z2#0',
                     'authored/fire_from_the_earth_mini_world_change'),)
        with mock.patch.object(explore,'BINDINGS',bindings):
            callback, = explore.callbacks(ROOT)
            self.assertEqual(callback['world_change'],'fire_from_the_earth')
            self.assertEqual(callback['admission'],{'premium':True})
            self.assertEqual(callback['selector']['native'], {
                'position':{'x':32072,'y':32604,'floor':-2},'ordinal':0,'appearance':4401})
            self.test_declared_area_point_really_has_the_retained_native_ground()

    def test_deeplings_navigator_approach_preserves_current_world_and_premium(self):
        bindings = (('oteryn:quest.deeplings_worldchange','s1',
                     'source-approach:deeplings-navigator-hub','chosen-source/deeplings_worldchange'),)
        with mock.patch.object(explore,'BINDINGS',bindings):
            callback, = explore.callbacks(ROOT)
            self.assertEqual(callback['world_change'],'deeplings')
            self.assertEqual(callback['admission'],{'premium':True})
            self.assertEqual(callback['selector']['native'], {
                'position':{'x':33641,'y':31379,'floor':-9},'ordinal':0,'appearance':8287})
            self.test_declared_area_point_really_has_the_retained_native_ground()

    def test_lion_trophy_entry_is_activated_with_its_completion_guard(self):
        callbacks = [c for c in explore.callbacks(ROOT)
                     if c['quest']=='oteryn:quest.lion_trophy_quest']
        self.assertEqual(len(callbacks),1,'guarded Lion Trophy entry is activated')
        self.assertEqual(callbacks[0]['required_completed_quests'],
                         ['oteryn:quest.the_ice_islands_quest'])
        self.assertEqual(callbacks[0]['admission'],{'premium':True})

    def test_elvenbane_dragon_floor_is_activated(self):
        callbacks = [c for c in explore.callbacks(ROOT)
                     if c['quest']=='oteryn:quest.authored.elvenbane_quest' and '/s2/' in c['key']]
        self.assertEqual(len(callbacks),1,'later dragon-floor stage must be bound')

    def test_elvenbane_dragon_floor_requires_its_first_explore_stage(self):
        bindings = (('oteryn:quest.authored.elvenbane_quest','s2',
                     'spawn-approach:oteryn:spawn.x32590_y31645_z5#0','authored/elvenbane_quest'),)
        with mock.patch.object(explore,'BINDINGS',bindings):
            callback, = explore.callbacks(ROOT)
            self.assertEqual(callback['selector']['native'], {
                'position':{'x':32591,'y':31645,'floor':-5},'ordinal':0,'appearance':436})
            self.assertIn({'track':'oteryn:quest-progress/authored/elvenbane_quest/s1',
                           'minimum':1,'maximum':1,'set':1,'preserve':True},callback['storage'])
            self.test_declared_area_point_really_has_the_retained_native_ground()

    def test_hero_rathleton_first_voice_entry_is_activated(self):
        callbacks = [c for c in explore.callbacks(ROOT)
                     if c['quest']=='oteryn:quest.hero_of_rathleton_quest' and '/s1/' in c['key']]
        self.assertEqual(len(callbacks),1,'the first voice approach is activated')
        self.assertEqual(callbacks[0]['admission'],{'premium':True})

    def test_hero_rathleton_voice_approach_precedes_its_access_door(self):
        bindings = (('oteryn:quest.hero_of_rathleton_quest','s1',
                     'npc-approach:oteryn:npc.strange_pipe','chosen-source/hero_of_rathleton_quest'),)
        with mock.patch.object(explore,'BINDINGS',bindings):
            callback, = explore.callbacks(ROOT)
            self.assertEqual(callback['selector']['native'], {
                'position':{'x':33533,'y':31951,'floor':-14},'ordinal':0,'appearance':20545})
            self.assertEqual(callback['admission'],{'premium':True})
            self.test_declared_area_point_really_has_the_retained_native_ground()

    def test_ghoul_room_outside_door_entry_is_activated(self):
        callbacks = [c for c in explore.callbacks(ROOT) if c['quest']=='oteryn:quest.ghoul_room_quest']
        self.assertEqual(len(callbacks),1,'outside final-key-door approach is activated')
        self.assertFalse(callbacks[0]['passages'])

    def test_ghoul_room_approach_is_outside_its_final_key_door(self):
        bindings = (('oteryn:quest.ghoul_room_quest','s1',
                     'door-approach:canary:door-gate/key/3600','chosen-source/ghoul_room_quest'),)
        with mock.patch.object(explore,'BINDINGS',bindings):
            callback, = explore.callbacks(ROOT)
            self.assertEqual(callback['selector']['native'], {
                'position':{'x':32507,'y':32175,'floor':-14},'ordinal':0,'appearance':409})
            self.assertFalse(callback['passages'])
            self.assertFalse(callback['offerings'])
            self.test_declared_area_point_really_has_the_retained_native_ground()

    def test_bear_room_basement_entry_is_activated(self):
        callbacks = [c for c in explore.callbacks(ROOT) if c['quest']=='oteryn:quest.bear_room_quest']
        self.assertEqual(len(callbacks),1,'outside-door basement approach is activated')
        self.assertFalse(callbacks[0]['passages'])
        self.assertFalse(callbacks[0].get('effects'))

    def test_bear_room_approach_precedes_the_stone_and_key_door(self):
        bindings = (('oteryn:quest.bear_room_quest','s1',
                     'door-approach:canary:door-gate/key/4601','chosen-source/bear_room_quest'),)
        with mock.patch.object(explore,'BINDINGS',bindings):
            callback, = explore.callbacks(ROOT)
            self.assertEqual(callback['selector']['native'], {
                'position':{'x':32145,'y':32102,'floor':-11},'ordinal':0,'appearance':355})
            self.assertFalse(callback['passages'])
            self.assertFalse(callback['offerings'])
            self.test_declared_area_point_really_has_the_retained_native_ground()

    def test_mad_mage_prison_entry_is_activated(self):
        callbacks = [c for c in explore.callbacks(ROOT)
                     if c['quest'] == 'oteryn:quest.mad_mage_room_quest']
        self.assertEqual(len(callbacks),1,'the outside-cell approach is activated')
        self.assertEqual(callbacks[0]['minimum_level'],40)

    def test_mad_mage_prison_approach_stays_outside_the_locked_cell(self):
        bindings = (('oteryn:quest.mad_mage_room_quest', 's1',
                     'npc-approach:oteryn:npc.a_prisoner', 'chosen-source/mad_mage_room_quest'),)
        with mock.patch.object(explore, 'BINDINGS', bindings):
            callback, = explore.callbacks(ROOT)
            self.assertEqual(callback['selector']['native'], {
                'position': {'x':32396,'y':32138,'floor':-13}, 'ordinal':0,'appearance':408})
            self.assertEqual(callback['minimum_level'],40)
            self.assertNotIn('admission',callback)
            self.test_declared_area_point_really_has_the_retained_native_ground()

    def test_crypt_distance_exception_does_not_widen_other_claims(self):
        import questpc_fragment_approaches as fragments
        key = 'canary:reward-claim/quest/u8_0/shardsof_ancient_winters/okolnir'
        x,y,floor,appearance = fragments.APPROACHES[key]
        with mock.patch.dict(fragments.APPROACHES, {key: (x+1,y,floor,appearance)}):
            with self.assertRaisesRegex(ValueError, 'declared distance'):
                fragments.resolve(ROOT, 'oteryn:quest.frost_dragon_quest', 'claim:' + key)
        key = 'canary:reward-claim/quest/u8_0/shardsof_ancient_winters/inukaya'
        with mock.patch.dict(fragments.DISTANCES, {key: 3}):
            with self.assertRaisesRegex(ValueError, 'declared distance'):
                fragments.resolve(ROOT, 'oteryn:quest.inukaya_quest', 'claim:' + key)

    def test_inukaya_and_medusa_entries_are_activated(self):
        callbacks = explore.callbacks(ROOT)
        for quest in ('oteryn:quest.inukaya_quest', 'oteryn:quest.medusa_shield_quest'):
            with self.subTest(quest=quest):
                self.assertEqual(sum(c['quest'] == quest for c in callbacks), 1,
                                 'one explicit crypt approach is activated')

    def test_inukaya_and_medusa_source_approaches_keep_their_requirements(self):
        bindings = (
            ('oteryn:quest.inukaya_quest', 's1',
             'claim:canary:reward-claim/quest/u8_0/shardsof_ancient_winters/inukaya',
             'chosen-source/inukaya_quest'),
            ('oteryn:quest.medusa_shield_quest', 's1',
             'claim:canary:reward-claim/quest/u7_3/the_medusa_quest',
             'chosen-source/medusa_shield_quest'),
        )
        with mock.patch.object(explore, 'BINDINGS', bindings):
            callbacks = explore.callbacks(ROOT)
            self.assertEqual([c['selector']['position'] for c in callbacks], [
                dict(x=32484,y=31109,floor=-8), dict(x=33050,y=32398,floor=-10)])
            self.assertTrue(all(c['admission'] == {'premium': True} for c in callbacks))
            self.assertEqual(callbacks[1]['minimum_level'], 60)
            self.test_declared_area_point_really_has_the_retained_native_ground()

    def test_world_entry_owner_cannot_be_missing_held_or_ambiguous(self):
        import questpc_world_requirements as world
        quest = 'oteryn:quest.grimvale_mini_world_change'
        for owners in ({}, {quest: [{'status': 'QUALIFIED', 'key': None}]},
                       {quest: [{'status': 'QUALIFIED', 'key': ''}]},
                       {quest: [{'status': 'HELD', 'key': 'grimvale'}]},
                       {quest: [{'status': 'QUALIFIED', 'key': 'grimvale'}] * 2}):
            with self.subTest(owners=owners):
                with self.assertRaises(ValueError):
                    world.requirement(quest, owners)
        self.assertIsNone(world.requirement('oteryn:quest.longsword_quest', {}))

    def test_grimvale_moon_entry_is_activated(self):
        callbacks = [c for c in explore.callbacks(ROOT)
                     if c['quest'] == 'oteryn:quest.grimvale_mini_world_change']
        self.assertEqual(len(callbacks), 1, 'current-moon entry is activated')
        self.assertEqual(callbacks[0]['world_change'], 'grimvale')

    def test_grimvale_explore_requires_the_current_full_moon_world_owner(self):
        binding = (('oteryn:quest.grimvale_mini_world_change', 's1',
                    'area-approach:oteryn:area.hunting_place.grimvale',
                    'chosen-source/grimvale_mini_world_change'),)
        with mock.patch.object(explore, 'BINDINGS', binding):
            callback, = explore.callbacks(ROOT)
            self.assertEqual(callback.get('world_change'), 'grimvale')
            self.assertEqual(callback['admission'], {'premium': True})
            self.assertEqual(callback['selector']['position'], dict(x=33335,y=31690,floor=-7))
            self.test_declared_area_point_really_has_the_retained_native_ground()

    def test_emperor_cookie_passage_entry_is_activated(self):
        callbacks = [c for c in explore.callbacks(ROOT)
                     if c['quest'] == 'oteryn:quest.emperor_s_cookies_quest']
        self.assertEqual(len(callbacks), 1, 'the pre-key passage entry is activated')
        self.assertTrue(callbacks[0]['key'].endswith('/emperor-s-cookies/s1/id3800'))

    def test_emperor_cookie_entry_reaches_key3800_before_its_locked_chamber(self):
        binding = (('oteryn:quest.emperor_s_cookies_quest', 's1',
                    'claim:canary:reward-claim/quest/key/id3800',
                    'chosen-source/emperor_s_cookies_quest'),)
        with mock.patch.object(explore, 'BINDINGS', binding):
            callback, = explore.callbacks(ROOT)
            self.assertEqual(callback['selector']['native'], {
                'position': {'x': 32605, 'y': 31907, 'floor': -3},
                'ordinal': 0, 'appearance': 436})
            self.assertNotIn('admission', callback)
            self.assertFalse(callback['offerings'])
            self.test_declared_area_point_really_has_the_retained_native_ground()

    def test_reused_definition_cannot_bypass_the_source_quest_owner(self):
        import copy
        import questpc_source_approaches as source
        import questpc_fragment_approaches as fragments
        cases = ((source, source.QUEST, 'source-approach:ferumbras-entrance'),
                 (fragments, 'oteryn:quest.frost_dragon_quest',
                  'claim:canary:reward-claim/quest/u8_0/shardsof_ancient_winters/okolnir'))
        definitions = [r['definition']
                       for path in (ROOT / 'content/quests/definitions').glob('quests-*.json')
                       for r in json.loads(path.read_text())['records']]
        for resolver, quest, reference in cases:
            with self.subTest(quest=quest):
                definition = next(d for d in definitions if d['identity']['key'] == quest)
                self.assertEqual(resolver.resolve(ROOT, quest, reference),
                                 resolver.resolve(ROOT, quest, reference, definition=definition))
                foreign = copy.deepcopy(definition)
                foreign['identity']['key'] = 'oteryn:quest.longsword_quest'
                with self.assertRaises(ValueError):
                    resolver.resolve(ROOT, quest, reference, definition=foreign)

    def test_grave_danger_twelve_visit_stage_is_activated(self):
        quest_callbacks = [c for c in explore.callbacks(ROOT)
                           if c['quest'] == 'oteryn:quest.grave_danger_quest']
        callbacks = [c for c in quest_callbacks if '/s1/' in c['key']]
        self.assertEqual(len(quest_callbacks), 13, 'twelve graves plus one guarded Cobra visit')
        self.assertEqual(len(callbacks), 12, 'all twelve chosen grave visits are activated atomically')
        self.assertEqual(len({c['key'] for c in callbacks}), 12)

    def test_script_approach_distance_exception_is_bounded(self):
        import questpc_source_approaches as source
        key = 'grave-danger/kilmaresh'
        quest, owner, script, origin, approach = source.SITES[key]
        invalid = (origin[0] + 5, origin[1], origin[2], approach[3])
        with mock.patch.dict(source.SITES, {key: (quest, owner, script, origin, invalid)}):
            with mock.patch.dict(source.DISTANCES, {key: 5}):
                with self.assertRaisesRegex(ValueError, 'declared distance'):
                    source.resolve(ROOT, quest, 'source-approach:' + key)
        invalid = (*approach[:2], approach[2] + 1, approach[3])
        with mock.patch.dict(source.SITES, {key: (quest, owner, script, origin, invalid)}):
            with self.assertRaisesRegex(ValueError, 'original floor'):
                source.resolve(ROOT, quest, 'source-approach:' + key)

    def test_grave_danger_twelve_distinct_native_visits_share_one_counter(self):
        names = ('dark-cathedral', 'femor-hills', 'ankrahmun', 'vengoth', 'orclands',
                 'ice-islands', 'kilmaresh', 'duke-krule', 'count-vlarkorth',
                 'earl-osam', 'lord-azaram', 'baeloc-nictros')
        bindings = tuple(('oteryn:quest.grave_danger_quest', 's1',
                          'source-approach:grave-danger/' + name,
                          'chosen-source/grave_danger_quest') for name in names)
        with mock.patch.object(explore, 'BINDINGS', bindings):
            callbacks = explore.callbacks(ROOT)
            self.assertEqual(len(callbacks), 12)
            self.assertEqual(len({c['key'] for c in callbacks}), 12)
            self.assertEqual(len({c['storage'][0]['track'] for c in callbacks}), 12)
            for callback in callbacks:
                self.assertEqual(callback['minimum_level'], 250)
                self.assertEqual(callback['admission'], {'premium': True})
                self.assertEqual(callback['storage'][1]['maximum'], 11)
                self.assertEqual(callback['storage'][1]['track'],
                                 'oteryn:quest-progress/chosen-source/grave_danger_quest/s1')
            self.test_declared_area_point_really_has_the_retained_native_ground()

    def test_two_pinned_script_destination_entries_are_activated(self):
        callbacks = explore.callbacks(ROOT)
        for quest in ('oteryn:quest.ferumbras_ascension_quest',
                      'oteryn:quest.heart_of_destruction_quest'):
            with self.subTest(quest=quest):
                entries = [c for c in callbacks if c['quest'] == quest]
                self.assertEqual(len(entries), 1, 'one pinned script destination is activated')
                self.assertEqual(entries[0]['minimum_level'], 150)
                self.assertEqual(entries[0]['admission'], {'premium': True})

    def test_heart_of_destruction_binds_the_zao_breach_destination(self):
        binding = (('oteryn:quest.heart_of_destruction_quest', 's1',
                    'source-approach:heart-zao-breach', 'chosen-source/heart_of_destruction_quest'),)
        with mock.patch.object(explore, 'BINDINGS', binding):
            callback, = explore.callbacks(ROOT)
            self.assertEqual(callback['selector']['native'], {
                'position': {'x': 32118, 'y': 31355, 'floor': -13},
                'ordinal': 0, 'appearance': 23054})
            self.assertEqual(callback['minimum_level'], 150)
            self.assertEqual(callback['admission'], {'premium': True})
            self.assertIsNone(callback['teleport'])
            self.test_declared_area_point_really_has_the_retained_native_ground()

    def test_script_destination_rejects_a_changed_source_revision(self):
        import tempfile
        import questpc_source_approaches as source
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            definitions = root / 'content/quests/definitions'
            definitions.mkdir(parents=True)
            definition = {'identity': {'key': source.QUEST}, 'source_refs': {'quest': {
                'key': 'canary:quest/ferumbras_ascension_quest', 'revision': 'changed-source'}}}
            (definitions / 'quests-00000-00000.json').write_text(
                json.dumps({'records': [{'definition': definition}]}))
            with self.assertRaisesRegex(ValueError, 'source approach quest owner changed'):
                source.resolve(root, source.QUEST, 'source-approach:ferumbras-entrance')

    def test_script_destination_approach_rejects_foreign_owners(self):
        import questpc_source_approaches as source
        for quest, reference in ((source.QUEST, 'source-approach:unknown'),
                                 (source.QUEST, 'ferumbras-entrance'),
                                 ('oteryn:quest.longsword_quest', 'source-approach:ferumbras-entrance')):
            with self.subTest(quest=quest, reference=reference):
                with self.assertRaisesRegex(ValueError, 'exact quest'):
                    source.resolve(ROOT, quest, reference)

    def test_ferumbras_arrival_approach_keeps_level_and_premium(self):
        binding = (('oteryn:quest.ferumbras_ascension_quest', 's1',
                    'source-approach:ferumbras-entrance', 'chosen-source/ferumbras_ascension_quest'),)
        with mock.patch.object(explore, 'BINDINGS', binding):
            callback, = explore.callbacks(ROOT)
            self.assertEqual(callback['selector']['native'], {
                'position': {'x': 33276, 'y': 32390, 'floor': -8},
                'ordinal': 0, 'appearance': 410})
            self.assertEqual(callback['minimum_level'], 150)
            self.assertEqual(callback['admission'], {'premium': True})
            self.assertIsNone(callback['teleport'])
            self.test_declared_area_point_really_has_the_retained_native_ground()

    def test_dragon_corpse_cave_entry_is_activated(self):
        callbacks = [c for c in explore.callbacks(ROOT)
                     if c['quest'] == 'oteryn:quest.dragon_corpse_quest']
        self.assertEqual(len(callbacks), 2, 'First cave and later corpse approaches are bound')
        self.assertEqual(callbacks[0]['key'], 'oteryn:quest-callback/m01b/dragon-corpse/s1/entry')
        self.assertEqual(callbacks[1]['key'], 'oteryn:quest-callback/m01b/dragon-corpse/s3/dragoncorpse')

    def test_dragon_corpse_entry_precedes_the_lower_shovel_route(self):
        binding = (('oteryn:quest.dragon_corpse_quest', 's1',
                    'area-approach:oteryn:area.hunting_place.rookgaard_bear_cave',
                    'chosen-source/dragon_corpse_quest'),)
        with mock.patch.object(explore, 'BINDINGS', binding):
            callback, = explore.callbacks(ROOT)
            self.assertEqual(callback['selector']['native'], {
                'position': {'x': 32147, 'y': 32207, 'floor': -7},
                'ordinal': 0, 'appearance': 4409})
            self.assertNotIn('admission', callback)
            self.assertIsNone(callback['teleport'])
            self.assertFalse(callback['offerings'])
            self.test_declared_area_point_really_has_the_retained_native_ground()

    def test_fishing_and_berserker_entries_are_activated(self):
        callbacks = explore.callbacks(ROOT)
        for quest in ('oteryn:quest.fishing_box_quest', 'oteryn:quest.berserker_treasure_quest'):
            with self.subTest(quest=quest):
                self.assertEqual(sum(c['quest'] == quest for c in callbacks), 1,
                                 'one retained entry is activated per quest')

    def test_fishing_and_berserker_approaches_keep_their_own_placements(self):
        bindings = (
            ('oteryn:quest.fishing_box_quest', 's1', 'claim:canary:reward-claim/quest/u8_0/fishing_box',
             'chosen-source/fishing_box_quest'),
            ('oteryn:quest.berserker_treasure_quest', 's1',
             'claim:canary:reward-claim/quest/u6_4/berserker_treasure/rewards/white_pearls',
             'chosen-source/berserker_treasure_quest'),
        )
        with mock.patch.object(explore, 'BINDINGS', bindings):
            callbacks = explore.callbacks(ROOT)
            self.assertEqual([c['selector']['position'] for c in callbacks], [
                dict(x=32016, y=31409, floor=-7), dict(x=33198, y=31923, floor=-11)])
            self.assertTrue(all(c['admission'] == {'premium': True} for c in callbacks))
            self.test_declared_area_point_really_has_the_retained_native_ground()

    def test_minotaur_hell_entry_reaches_its_retained_deep_room(self):
        binding = (('oteryn:quest.minotaur_hell_quest', 's1',
                    'claim:canary:reward-claim/kv/minohell1', 'chosen-source/minotaur_hell_quest'),)
        with mock.patch.object(explore, 'BINDINGS', binding):
            callback, = explore.callbacks(ROOT)
            self.assertEqual(callback['selector']['native'], {
                'position': {'x': 32131, 'y': 32066, 'floor': -12},
                'ordinal': 0, 'appearance': 351})
            self.test_declared_area_point_really_has_the_retained_native_ground()

    def test_katana_entry_reaches_the_key_cache_before_the_locked_room(self):
        binding = (('oteryn:quest.katana_quest', 's1',
                    'claim:canary:reward-claim/quest/key/id4603', 'chosen-source/katana_quest'),)
        with mock.patch.object(explore, 'BINDINGS', binding):
            callback, = explore.callbacks(ROOT)
            self.assertEqual(callback['selector']['native'], {
                'position': {'x': 32177, 'y': 32132, 'floor': -9},
                'ordinal': 0, 'appearance': 355})
            self.test_declared_area_point_really_has_the_retained_native_ground()

    def test_camp_and_temple_entries_are_activated(self):
        callbacks = explore.callbacks(ROOT)
        for quest in ('oteryn:quest.iron_helmet_quest', 'oteryn:quest.goblin_temple_quest'):
            with self.subTest(quest=quest):
                self.assertTrue(any(c['quest'] == quest for c in callbacks),
                                'camp or temple entry is activated')

    def test_goblin_temple_entry_is_beside_its_retained_chest(self):
        binding = (('oteryn:quest.goblin_temple_quest', 's1',
                    'claim:canary:reward-claim/kv/goblintemple', 'chosen-source/goblin_temple_quest'),)
        with mock.patch.object(explore, 'BINDINGS', binding):
            callback, = explore.callbacks(ROOT)
            self.assertEqual(callback['selector']['native'], {
                'position': {'x': 31973, 'y': 32210, 'floor': -12},
                'ordinal': 0, 'appearance': 502})
            self.test_declared_area_point_really_has_the_retained_native_ground()

    def test_iron_helmet_entry_reaches_the_retained_porgol_camp(self):
        binding = (('oteryn:quest.iron_helmet_quest', 's1',
                    'claim:canary:reward-claim/kv/ironhelmet', 'chosen-source/iron_helmet_quest'),)
        with mock.patch.object(explore, 'BINDINGS', binding):
            callback, = explore.callbacks(ROOT)
            self.assertEqual(callback['selector']['native'], {
                'position': {'x': 32770, 'y': 32225, 'floor': -7},
                'ordinal': 0, 'appearance': 4516})
            self.test_declared_area_point_really_has_the_retained_native_ground()

    def test_frost_dragon_lair_entry_is_activated(self):
        callbacks = [c for c in explore.callbacks(ROOT) if c['quest'] == 'oteryn:quest.frost_dragon_quest']
        self.assertEqual(len(callbacks), 1, 'Frost Dragon lair entry is activated')
        self.assertEqual(callbacks[0]['selector']['position']['floor'], -8)

    def test_frost_dragon_approach_is_beside_the_retained_quest_corpse(self):
        binding = (('oteryn:quest.frost_dragon_quest', 's1',
                    'claim:canary:reward-claim/quest/u8_0/shardsof_ancient_winters/okolnir',
                    'chosen-source/frost_dragon_quest'),)
        with mock.patch.object(explore, 'BINDINGS', binding):
            callback, = explore.callbacks(ROOT)
            self.assertEqual(callback['selector']['native'], {
                'position': {'x': 32146, 'y': 31431, 'floor': -8},
                'ordinal': 0, 'appearance': 4407})
            self.assertEqual(callback['admission'], {'premium': True})
            self.test_declared_area_point_really_has_the_retained_native_ground()
        foreign = (('oteryn:quest.longsword_quest', 's1', binding[0][2],
                    'chosen-source/longsword_quest'),)
        with mock.patch.object(explore, 'BINDINGS', foreign):
            with self.assertRaisesRegex(ValueError, 'same canonical quest'):
                explore.callbacks(ROOT)

    def test_behemoth_floor_entry_is_activated(self):
        callbacks = [c for c in explore.callbacks(ROOT) if c['quest'] == 'oteryn:quest.behemoth_quest']
        self.assertEqual(len(callbacks), 1, 'Behemoth floor entry is activated')
        self.assertEqual(callbacks[0]['selector']['position']['floor'], -12)

    def test_spawn_approach_rejects_another_coordinate_frame(self):
        import questpc_spawn_approaches as spawns
        source = ROOT / spawns.SOURCE
        read = Path.read_text
        def other_frame(path, *args, **kwargs):
            raw = read(path, *args, **kwargs)
            if path == source:
                document = json.loads(raw)
                document['coordinate_frame'] = 'another-world-frame'
                return json.dumps(document)
            return raw
        with mock.patch.object(Path, 'read_text', other_frame):
            with self.assertRaisesRegex(ValueError, 'coordinate frame'):
                spawns.resolve(ROOT, 'oteryn:quest.behemoth_quest',
                               'spawn-approach:oteryn:spawn.x33230_y31696_z12#3')

    def test_behemoth_visit_reaches_the_retained_behemoth_floor(self):
        reference = 'spawn-approach:oteryn:spawn.x33230_y31696_z12#3'
        binding = (('oteryn:quest.behemoth_quest', 's1', reference, 'chosen-source/behemoth_quest'),)
        with mock.patch.object(explore, 'BINDINGS', binding):
            callback, = explore.callbacks(ROOT)
            self.assertEqual(callback['selector']['native'], {
                'position': {'x': 33235, 'y': 31695, 'floor': -12},
                'ordinal': 0, 'appearance': 499})
            self.assertEqual(callback['minimum_level'], 80)
            self.assertEqual(callback['admission'], {'premium': True})
            self.test_declared_area_point_really_has_the_retained_native_ground()
        foreign = (('oteryn:quest.dragon_tower_quest', 's1', reference,
                    'chosen-source/dragon_tower_quest'),)
        with mock.patch.object(explore, 'BINDINGS', foreign):
            with self.assertRaisesRegex(ValueError, 'same quest'):
                explore.callbacks(ROOT)

    def test_dragon_tower_and_berserk_potion_approaches_are_activated(self):
        callbacks = explore.callbacks(ROOT)
        for quest in ('oteryn:quest.dragon_tower_quest', 'oteryn:quest.berserk_potion_quest'):
            with self.subTest(quest=quest):
                self.assertTrue(any(c['quest'] == quest for c in callbacks),
                                'chosen adjacent Area visit is activated')

    def test_area_approaches_reject_changed_markers_and_nonadjacent_tiles(self):
        import questpc_area_approaches as approaches
        key = 'oteryn:area.hunting_place.shadowthorn'
        origin, anchor = approaches.APPROACHES[key]
        cases = (
            (((origin[0] + 1, origin[1], origin[2]), anchor), 'marker changed'),
            ((origin, (anchor[0], anchor[1] - 1, anchor[2], anchor[3])), 'declared distance'),
        )
        for replacement, error in cases:
            with mock.patch.dict(approaches.APPROACHES, {key: replacement}):
                with self.assertRaisesRegex(ValueError, error):
                    approaches.resolve(ROOT, 'area-approach:' + key)

    def test_chosen_area_approaches_keep_the_pinned_marker_unchanged(self):
        bindings = (
            ('oteryn:quest.dragon_tower_quest', 's1',
             'area-approach:oteryn:area.hunting_place.shadowthorn', 'chosen-source/dragon_tower_quest'),
            ('oteryn:quest.berserk_potion_quest', 's1',
             'area-approach:oteryn:area.hunting_place.helheim', 'chosen-source/berserk_potion_quest'),
        )
        before = (ROOT / explore.AREA).read_bytes()
        with mock.patch.object(explore, 'BINDINGS', bindings):
            callbacks = explore.callbacks(ROOT)
            self.assertEqual([c['selector']['position'] for c in callbacks], [
                dict(x=33075, y=32169, floor=-7), dict(x=32479, y=31179, floor=-7)])
            self.test_declared_area_point_really_has_the_retained_native_ground()
        self.assertEqual((ROOT / explore.AREA).read_bytes(), before)

    def test_fire_axe_entry_is_active_with_level_sixty_and_current_premium(self):
        callback = next((c for c in explore.callbacks(ROOT)
                         if c['quest'] == 'oteryn:quest.fire_axe_quest'), None)
        self.assertIsNotNone(callback, 'Fire Axe EXPLORE entry is bound')
        self.assertEqual(callback['minimum_level'], 60)
        self.assertEqual(callback['admission'], {'premium': True})

    def test_koshei_activation_contains_all_four_fragment_visits(self):
        callbacks = [c for c in explore.callbacks(ROOT)
                     if c['quest'] == 'oteryn:quest.koshei_the_deathless_quest']
        self.assertEqual(len(callbacks), 4, 'all four Koshei visits are activated together')
        self.assertEqual({c['storage'][1]['maximum'] for c in callbacks}, {3})

    def test_complete_koshei_fragment_set_has_four_distinct_native_visits(self):
        bindings = tuple(('oteryn:quest.koshei_the_deathless_quest', 's1',
                          'claim:canary:reward-claim/quest/u8_1/koshei_the_deathless/'
                          + 'koshei_amulet_part' + str(part),
                          'chosen-source/koshei_the_deathless_quest') for part in range(1, 5))
        with mock.patch.object(explore, 'BINDINGS', bindings):
            callbacks = explore.callbacks(ROOT)
            self.assertEqual(len(callbacks), 4)
            self.assertEqual(len({c['key'] for c in callbacks}), 4)
            self.assertEqual(len({c['storage'][0]['track'] for c in callbacks}), 4)
            self.assertEqual({c['storage'][1]['maximum'] for c in callbacks}, {3})
            self.assertTrue(all(c['minimum_level'] == 40 for c in callbacks))
            self.test_declared_area_point_really_has_the_retained_native_ground()
        for invalid in (bindings[:3], (*bindings[:3], bindings[0])):
            with mock.patch.object(explore, 'BINDINGS', invalid):
                with self.assertRaisesRegex(ValueError, 'complete visit anchor set'):
                    explore.callbacks(ROOT)

    def test_fragment_approach_cannot_borrow_a_foreign_quests_claim(self):
        import questpc_fragment_approaches as fragments
        with self.assertRaisesRegex(ValueError, 'same canonical quest'):
            fragments.resolve(ROOT, 'oteryn:quest.longsword_quest',
                              'claim:' + fragments.PREFIX + '1')

    def test_edron_goblin_and_lions_rock_keep_current_premium(self):
        for quest, x, y, appearance in (
                ('oteryn:quest.edron_goblin_quest', 33108, 31850, 355),
                ('oteryn:quest.lions_rock', 33138, 32358, 4399)):
            with self.subTest(quest=quest):
                callback = next((c for c in explore.callbacks(ROOT) if c['quest'] == quest), None)
                self.assertIsNotNone(callback, 'native Premium EXPLORE entry is bound')
                self.assertEqual(callback['selector']['native'], {
                    'position': {'x': x, 'y': y, 'floor': -7},
                    'ordinal': 0, 'appearance': appearance})
                self.assertEqual(callback['admission'], {'premium': True})

    def test_elvenbane_first_stage_uses_the_canonical_clear_entry(self):
        callback = next((c for c in explore.callbacks(ROOT)
                         if c['quest'] == 'oteryn:quest.authored.elvenbane_quest'), None)
        self.assertIsNotNone(callback, 'Elvenbane native EXPLORE entry is bound')
        self.assertEqual(callback['selector']['native'], {
            'position': {'x': 32590, 'y': 31645, 'floor': -7},
            'ordinal': 0, 'appearance': 436})
        self.assertEqual(callback['storage'][1]['track'],
                         'oteryn:quest-progress/authored/elvenbane_quest/s1')
        self.assertNotIn('admission', callback)

    def test_source_portal_approach_keeps_the_original_teleport_separate(self):
        bindings = (('oteryn:quest.alawar_s_vault_quest', 's1',
                     'callback:oteryn:quest-callback/alawars-vault-entrance',
                     'chosen-source/alawar_s_vault_quest'),)
        before = (ROOT / 'content/quests/world-callbacks.json').read_bytes()
        with mock.patch.object(explore, 'BINDINGS', bindings):
            callback, = explore.callbacks(ROOT)
        self.assertEqual(callback['selector']['native'], {
            'position': {'x': 32187, 'y': 31623, 'floor': -8},
            'ordinal': 0, 'appearance': 409})
        self.assertIsNone(callback['teleport'])
        self.assertIn('content/quests/world-callbacks.json#/callbacks/0', callback['sources'])
        self.assertEqual((ROOT / 'content/quests/world-callbacks.json').read_bytes(), before)

    def test_approach_cannot_borrow_another_quests_source_entry(self):
        bindings = (('oteryn:quest.longsword_quest', 's1',
                     'callback:oteryn:quest-callback/alawars-vault-entrance',
                     'chosen-source/longsword_quest'),)
        with mock.patch.object(explore, 'BINDINGS', bindings):
            with self.assertRaisesRegex(ValueError, 'same quest entry'):
                explore.callbacks(ROOT)

    def test_later_explore_preserves_each_preceding_chosen_stage(self):
        quest = 'oteryn:quest.dragon_corpse_quest'
        storage = explore.stage_storage(quest, 's3', canonical_recipe(quest),
                                        'chosen-source/dragon_corpse_quest', 'corridor')
        prefix = 'oteryn:quest-progress/chosen-source/dragon_corpse_quest/'
        self.assertEqual(storage[2:], [
            dict(track=prefix + stage, minimum=1, maximum=1, set=1, preserve=True)
            for stage in ('s1', 's2')])
        self.assertEqual(storage[0]['track'], prefix + 's3/visited/corridor')

    def test_distinct_koshei_visits_share_a_bounded_counter(self):
        quest = 'oteryn:quest.koshei_the_deathless_quest'
        recipe = canonical_recipe(quest)
        first = explore.stage_storage(quest, 's1', recipe,
                                      'chosen-source/koshei_the_deathless_quest', 'fragment-1')
        second = explore.stage_storage(quest, 's1', recipe,
                                       'chosen-source/koshei_the_deathless_quest', 'fragment-2')
        self.assertNotEqual(first[0]['track'], second[0]['track'])
        self.assertEqual(first[1], second[1])
        self.assertEqual((first[1]['minimum'], first[1]['maximum'], first[1]['set']),
                         (0, 3, 1))
        self.assertTrue(first[1]['increment'])
        self.assertEqual(len(first), 2)

    def test_premium_and_level_requirements_reach_the_fenced_callback(self):
        cases = (
            ('oteryn:quest.authored.asura_palace_quest', 'oteryn:area.hunting_place.asura_palace',
             'authored/asura_palace_quest', 0),
            ('oteryn:quest.fire_axe_quest', 'oteryn:area.hunting_place.edron_dragon_lair',
             'chosen-source/fire_axe_quest', 60),
        )
        for quest, area, tail, level in cases:
            with self.subTest(quest=quest), mock.patch.object(
                    explore, 'BINDINGS', ((quest, 's1', area, tail),)):
                callback, = explore.callbacks(ROOT)
                self.assertEqual(callback['admission'], {'premium': True})
                self.assertEqual(callback.get('minimum_level', 0), level)

    def test_non_explore_and_unknown_stages_are_rejected(self):
        for stage in ('s2', 'missing'):
            binding = tuple((q, stage, a, t) for q, _, a, t in explore.BINDINGS)
            with self.subTest(stage=stage), mock.patch.object(explore, 'BINDINGS', binding):
                with self.assertRaisesRegex(ValueError, 'one EXPLORE stage'):
                    explore.callbacks(ROOT)

    def test_counter_owner_must_match_the_canonical_quest(self):
        binding = tuple((q, s, a, 'chosen-source/foreign')
                        for q, s, a, _ in explore.BINDINGS)
        with mock.patch.object(explore, 'BINDINGS', binding):
            with self.assertRaisesRegex(ValueError, 'counter owner'):
                explore.callbacks(ROOT)

    def test_completed_ice_islands_guard_is_not_silently_dropped(self):
        guarded = (('oteryn:quest.lion_trophy_quest', 's1',
                    'oteryn:area.hunting_place.formorgar_mines', 'chosen-source/lion_trophy_quest'),)
        with mock.patch.object(explore, 'BINDINGS', guarded):
            callback, = explore.callbacks(ROOT)
            self.assertEqual(callback['required_completed_quests'],
                             ['oteryn:quest.the_ice_islands_quest'])
            self.assertEqual(callback['admission'], {'premium':True})
            self.assertEqual(callback['storage'][1]['track'],
                             'oteryn:quest-progress/chosen-source/lion_trophy_quest/s1')

    def test_declared_area_point_really_has_the_retained_native_ground(self):
        sys.path.insert(0, str(ROOT / 'tools/content-schema/world-authoring'))
        import world_region_codec as codec
        index = json.loads((ROOT / 'content/world/placements/index.json').read_text())
        for callback in explore.callbacks(ROOT):
            with self.subTest(quest=callback['quest']):
                native = callback['selector']['native']
                at = native['position']
                z, rx, ry = -at['floor'], at['x'] // 256, at['y'] // 256
                source = ROOT / f'content/world/placements/region-z{z:02}-x{rx}-y{ry}.b3'
                raw = source.read_bytes()
                floor, region_x, region_y, table = codec.parse_region(raw)
                self.assertEqual((floor, region_x, region_y), (z, rx, ry))
                found = []
                for local, offset, length in table:
                    sx, sy = rx * 8 + local % 8, ry * 8 + local // 8
                    if (sx, sy) != (at['x'] // 32, at['y'] // 32):
                        continue
                    payload = codec.zstandard.ZstdDecompressor().decompress(
                        raw[offset:offset + length], max_output_size=codec.MAX_SECTOR_BYTES)
                    for x, y, _, house, _, items in codec.decode_sector(payload, sx, sy):
                        if (x, y) == (at['x'], at['y']):
                            self.assertEqual(house, 0)
                            found = [index['palette'][palette]['source_item_id']
                                     for palette, depth, _ in items if depth == 0]
                self.assertEqual(found, [native['appearance']])

    def test_geomancer_uses_its_exact_canonical_area_and_counter(self):
        callback = next(c for c in explore.callbacks(ROOT)
                        if c['quest'] == 'oteryn:quest.geomancer_quest')
        self.assertEqual(callback['key'], 'oteryn:quest-callback/m01b/geomancer/s1/entry')
        self.assertEqual(callback['selector']['native'], {
            'position': {'x': 32418, 'y': 32017, 'floor': -7},
            'ordinal': 0, 'appearance': 4529})
        self.assertIn('https://tibia.fandom.com/index.php?curid=20335&oldid=1114489',
                      callback['sources'])
        self.assertEqual(callback['storage'][1]['track'],
                         'oteryn:quest-progress/chosen-source/geomancer_quest/s1')

    def test_native_anchor_has_no_value_or_movement_effect(self):
        callback = next(c for c in explore.callbacks(ROOT)
                        if c['quest'] == 'oteryn:quest.longsword_quest')
        self.assertEqual(callback['quest'], 'oteryn:quest.longsword_quest')
        self.assertEqual(callback['trigger'], 'step_in')
        self.assertEqual(callback['selector']['native'], {
            'position': {'x': 32663, 'y': 31970, 'floor': -7},
            'ordinal': 0, 'appearance': 4525})
        self.assertTrue(callback['requires_world_admission'])
        self.assertEqual(callback['offerings'], [])
        self.assertIsNone(callback['teleport'])
        seen, counter = callback['storage']
        self.assertEqual((seen['minimum'], seen['maximum'], seen['set']), (0, 0, 1))
        self.assertEqual(counter['track'],
                         'oteryn:quest-progress/chosen-source/longsword_quest/s1')
        self.assertEqual((counter['minimum'], counter['maximum'], counter['set']), (0, 0, 1))
        self.assertTrue(counter['increment'])

    def test_qm55_held_chakoya_preserves_recipe_without_an_entry_binding(self):
        import questpc_world_requirements as world
        quest = 'oteryn:quest.authored.chakoya_iceberg_mini_world_change'
        owners = world.catalogue(ROOT)
        self.assertEqual([(row['key'], row['status']) for row in owners[quest]],
                         [('chakoya_iceberg', 'HELD')])
        self.assertFalse(any(c['quest'] == quest for c in explore.callbacks(ROOT)))
        with self.assertRaisesRegex(ValueError, 'needs one admitted World owner'):
            world.requirement(quest, owners)
        recipe = canonical_recipe(quest)
        self.assertTrue(recipe['requirements']['premium'])
        visit, counter = explore.stage_storage(
            quest, 's1', recipe, 'authored/chakoya_iceberg_mini_world_change')
        track = 'oteryn:quest-progress/authored/chakoya_iceberg_mini_world_change/s1'
        self.assertEqual(visit['track'], track + '/visited/entry')
        self.assertEqual((visit['minimum'], visit['maximum'], visit['set']), (0, 0, 1))
        self.assertEqual(counter['track'], track)
        self.assertEqual((counter['minimum'], counter['maximum'], counter['set']), (0, 0, 1))
        self.assertTrue(counter['increment'])

    def test_qm55_admitted_premium_entry_preserves_once_only_progress(self):
        callback = next(c for c in explore.callbacks(ROOT)
                        if c['quest'] == 'oteryn:quest.authored.asura_palace_quest')
        self.assertEqual(callback['admission'], {'premium': True})
        self.assertEqual(callback['selector']['native'], {
            'position': {'x': 32953, 'y': 32685, 'floor': -7},
            'ordinal': 0, 'appearance': 923})
        self.assertTrue(callback['requires_world_admission'])
        visit, counter = callback['storage']
        track = 'oteryn:quest-progress/authored/asura_palace_quest/s1'
        self.assertEqual(visit['track'], track + '/visited/entry')
        self.assertEqual((visit['minimum'], visit['maximum'], visit['set']), (0, 0, 1))
        self.assertEqual(counter['track'], track)
        self.assertEqual((counter['minimum'], counter['maximum'], counter['set']), (0, 0, 1))
        self.assertTrue(counter['increment'])
        self.assertEqual(callback['offerings'], [])
        self.assertIsNone(callback['teleport'])


    def test_unimplemented_required_guard_is_not_silently_dropped(self):
        guarded = (('oteryn:quest.fury_gates_mini_world_change', 's1',
                    'oteryn:area.hunting_place.fury_gate', 'chosen-source/fury_gates_mini_world_change'),)
        with mock.patch.object(explore, 'BINDINGS', guarded):
            with self.assertRaisesRegex(ValueError, 'World-change EXPLORE needs one admitted World owner: oteryn:quest.fury_gates_mini_world_change'):
                explore.callbacks(ROOT)
