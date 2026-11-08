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
