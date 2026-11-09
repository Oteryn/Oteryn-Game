"""M03 binds live admission to retained native entrance objects."""
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import qengine_access

ROOT = Path(__file__).resolve().parents[3]


class WorldAdmissionTests(unittest.TestCase):
    def test_anniversary_landing_has_source_qualified_nonmaterializable_profiles(self):
        overlay = json.loads((ROOT / 'tools/content-schema/native-gameplay/floor-assembly-item-profiles.json').read_text())
        self.assertEqual(overlay['source_revision'], '201189f871d23597b82caf88fd611454a2ddefb8')
        profiles = {row['production_definition']['production_key']: row for row in overlay['records']}
        aggregate = json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_text())
        active = {row['production_definition']['production_key']: row for row in aggregate['records']}
        for item in (15534, 15620, 25100):
            profile = profiles['oteryn:item.tibia.i' + str(item)]
            self.assertIs(profile['attributes']['blocks_movement'], False)
            self.assertIs(profile['admission']['materializable'], False)
            generated = dict(active[profile['production_definition']['production_key']])
            generated['attributes'] = dict(generated['attributes'])
            # Registration adds client stack-order annotations after physical qualification.
            generated['attributes'].pop('always_on_top')
            generated['attributes'].pop('always_on_top_order')
            self.assertEqual(generated, profile)

    def callbacks(self):
        return [c for c in qengine_access.document()['callbacks']
                if c['key'].startswith('oteryn:quest-callback/qm03/')]

    def test_entrances_require_the_chosen_live_facts(self):
        callbacks = self.callbacks()
        self.assertEqual(len(callbacks), 10)
        anniversary = next(c for c in callbacks if c['key'].endswith('/anniversary'))
        pits = next(c for c in callbacks if c['key'].endswith('/pits'))
        self.assertEqual(anniversary['quest'], 'oteryn:quest.25_years_of_tibia_quest')
        self.assertEqual(anniversary['admission'], dict(regular_blessings=True, promotion=True, premium=True))
        self.assertEqual(anniversary['minimum_level'], 20)
        self.assertEqual(anniversary['trigger'], 'use')
        self.assertEqual(anniversary['teleport'], dict(x=33453, y=31030, floor=-8))
        self.assertEqual(pits['quest'], 'oteryn:quest.the_pits_of_inferno_quest')
        self.assertEqual(pits['admission'], dict(premium=True))
        self.assertEqual(pits['minimum_level'], 80)
        self.assertEqual(pits['trigger'], 'step_in')
        self.assertTrue(pits['return_on_failure'])
        for callback in callbacks:
            self.assertTrue(callback['requires_world_admission'])
            self.assertTrue(callback['storage'][0]['at_least'])
            self.assertEqual(callback['storage'][0]['set'], 1)

    def test_every_source_time_machine_tile_has_the_same_live_guards(self):
        rows = [c for c in self.callbacks() if c['quest'] == 'oteryn:quest.25_years_of_tibia_quest']
        self.assertEqual(len(rows), 9)
        self.assertEqual({(c['selector']['position']['x'], c['selector']['position']['y']) for c in rows},
                         {(x, y) for x in range(32429, 32432) for y in range(32164, 32167)})
        primary = next(c for c in rows if c['key'].endswith('/anniversary'))
        for row in rows:
            for field in ('admission', 'minimum_level', 'teleport', 'storage', 'requires_world_admission'):
                self.assertEqual(row[field], primary[field])

    def write_requirements(self, root, extra=None, promotion=True):
        directory = Path(root) / 'content/quests/definitions'
        directory.mkdir(parents=True)
        for shard, quest, requirements in (
            ('quests-00200-00299.json', '25_years_of_tibia_quest',
             dict(min_level=21, premium=True,
                  prerequisites=['All regular blessings'] + (['Vocation promotion'] if promotion else []))),
            ('quests-00000-00099.json', 'the_pits_of_inferno_quest',
             dict(min_level=80, premium=False, prerequisites=['Rope', 'Shovel'])),
        ):
            if extra and quest == '25_years_of_tibia_quest':
                requirements['prerequisites'].append(extra)
            definition = dict(identity=dict(key='oteryn:quest.' + quest),
                              oteryn_recipe=dict(payload=dict(recipe=dict(requirements=requirements))))
            (directory / shard).write_text(json.dumps(dict(records=[dict(definition=definition)])))

    def test_canonical_requirements_drive_the_emitted_admission(self):
        with tempfile.TemporaryDirectory() as directory:
            self.write_requirements(directory, promotion=False)
            with mock.patch.object(qengine_access, 'ROOT', Path(directory)), \
                    mock.patch('questpc_explore_bindings.callbacks', return_value=[]):
                rows = self.callbacks()
            anniversary = next(c for c in rows if c['key'].endswith('/anniversary'))
            pits = next(c for c in rows if c['key'].endswith('/pits'))
            self.assertEqual(anniversary['minimum_level'], 21)
            self.assertEqual(anniversary['admission'], dict(regular_blessings=True, premium=True))
            self.assertIsNone(pits.get('admission'))

    def test_unknown_canonical_admission_requirement_is_not_dropped(self):
        with tempfile.TemporaryDirectory() as directory:
            self.write_requirements(directory, extra='Unbound prerequisite')
            with mock.patch.object(qengine_access, 'ROOT', Path(directory)), \
                    mock.patch('questpc_explore_bindings.callbacks', return_value=[]):
                with self.assertRaisesRegex(ValueError, 'unsupported prerequisite'):
                    self.callbacks()

    def test_selectors_exist_in_retained_native_map(self):
        sys.path.insert(0, str(ROOT / 'tools/content-schema/world-authoring'))
        import world_region_codec as codec
        callbacks = self.callbacks()
        self.assertEqual(len(callbacks), 10)
        palette = json.loads((ROOT / 'content/world/placements/index.json').read_text())['palette']
        for callback in callbacks:
            native = callback['selector']['native']
            at = native['position']
            path = ROOT / f"content/world/placements/region-z{-at['floor']:02}-x{at['x']//256}-y{at['y']//256}.b3"
            raw = path.read_bytes()
            _, rx, ry, table = codec.parse_region(raw)
            found = None
            for local, offset, length in table:
                sx, sy = rx * 8 + local % 8, ry * 8 + local // 8
                if (sx, sy) != (at['x']//32, at['y']//32):
                    continue
                payload = codec.zstandard.ZstdDecompressor().decompress(raw[offset:offset+length], max_output_size=codec.MAX_SECTOR_BYTES)
                for x, y, _, house, _, items in codec.decode_sector(payload, sx, sy):
                    if (x, y) == (at['x'], at['y']):
                        self.assertEqual(house, 0)
                        found = [palette[p]['source_item_id'] for p, depth, _ in items if depth == 0]
            self.assertIsNotNone(found)
            self.assertEqual(found[native['ordinal']], native['appearance'])
