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

    def test_unimplemented_required_guard_is_not_silently_dropped(self):
        guarded = (('oteryn:quest.lion_trophy_quest', 's1',
                    'oteryn:area.hunting_place.formorgar_mines', 'chosen-source/lion_trophy_quest'),)
        with mock.patch.object(explore, 'BINDINGS', guarded):
            with self.assertRaisesRegex(ValueError, 'additional authoritative guards'):
                explore.callbacks(ROOT)

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
