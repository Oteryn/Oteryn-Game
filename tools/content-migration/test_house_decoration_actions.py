"""W18 source catalogue and native policy closure for ordinary House decoration USE."""
import copy
import json
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

from house_decoration_actions import (
    CATALOGUE,
    PROFILES,
    admit,
    apply_top_order,
    encode,
    sql_pairs,
)

ROOT = Path(__file__).resolve().parents[2]


class HouseDecorationActions(unittest.TestCase):
    def test_captured_source_families_are_complete_without_invented_targets(self):
        catalogue = json.loads((ROOT / CATALOGUE).read_bytes())
        profiles = json.loads((ROOT / PROFILES).read_bytes())
        self.assertEqual(len(catalogue['carpets']), 160)
        self.assertEqual(len(catalogue['windows']), 104)
        self.assertEqual(len(catalogue['music']), 33)
        self.assertEqual(len(catalogue['mirror']['item_ids']), 8)
        self.assertEqual(len(catalogue['magic_trees']), 8)
        self.assertEqual(len(catalogue['bed_kits']), 5)
        self.assertEqual(catalogue['mirror']['cooldown_ms'], 72_000_000)
        self.assertEqual(catalogue['fish_tank']['effect_binding'], 'canary.appearance:effect/plunging_fish')
        known = {row['production_definition']['production_key'] for row in profiles['records']}
        for field in ('carpets', 'windows', 'magic_trees', 'native_transforms'):
            for row in catalogue[field]:
                self.assertRegex(row['donor'], r'[^:]+:\d+')
                self.assertIn(f"oteryn:item.tibia.i{row['item_id']}", known)
                self.assertIn(f"oteryn:item.tibia.i{row['target']}", known)
        self.assertEqual({(row['item_id'], row['target']) for row in catalogue['unavailable_native_transforms']},
                         {(34276, 34277), (37782, 37883), (26099, 23397)})
        for row in catalogue['native_transforms']:
            if row.get('noop'):
                self.assertEqual(row['item_id'], row['target'])
                self.assertNotEqual(row['requested_target'], row['target'])
                self.assertEqual(row['engine_donor'], 'src/game/game.cpp:3054-3057')
        pairs = sql_pairs(catalogue)
        self.assertIn(b"('oteryn:item.tibia.i37365','oteryn:item.tibia.i37366')", pairs)
        self.assertIn(b"('oteryn:item.tibia.i2603','oteryn:item.tibia.i2603')", pairs)
        self.assertNotIn(b"'oteryn:item.tibia.i34277'", pairs)
        self.assertNotIn(b"'oteryn:item.tibia.i37883'", pairs)
        self.assertIn(b"('oteryn:item.tibia.i831','oteryn:item.tibia.i2491','oteryn:item.tibia.i734')", pairs)
        self.assertIn(b"('oteryn:item.tibia.i831','oteryn:item.tibia.i2492','oteryn:item.tibia.i735')", pairs)
        self.assertNotIn(b"('oteryn:item.tibia.i832','oteryn:item.tibia.i2491','oteryn:item.tibia.i734')", pairs)
        self.assertTrue(encode(catalogue).endswith(b'\n'))

    def test_admission_preserves_existing_specialized_facts_and_adds_closed_sources(self):
        catalogue = json.loads((ROOT / CATALOGUE).read_bytes())
        overlay = json.loads((ROOT / PROFILES).read_bytes())
        existing = copy.deepcopy(next(row for row in overlay['records']
                                      if row['production_definition']['production_key'] == 'oteryn:item.tibia.i2603'))
        existing['semantics']['physical']['value']['weight'] = {'state': 'KNOWN', 'value': 9999}
        with TemporaryDirectory() as directory:
            root = Path(directory)
            for path, document in ((CATALOGUE, catalogue), (PROFILES, overlay)):
                (root / path).parent.mkdir(parents=True, exist_ok=True)
                (root / path).write_bytes(encode(document))
            generated = {'items.json': encode({'records': [existing]}), 'manifest.json': encode({'item_profiles': {}})}
            admit(root, generated, 'items.json', 'manifest.json',
                  lambda row: row['production_definition']['production_key'], encode)
            profiles = {row['production_definition']['production_key']: row
                        for row in json.loads(generated['items.json'])['records']}
            self.assertEqual(profiles['oteryn:item.tibia.i2603']['semantics']['physical'], existing['semantics']['physical'])
            self.assertIn('oteryn:item.tibia.i37365', profiles)
            self.assertIsInstance(profiles['oteryn:item.tibia.i2603']['attributes']['always_on_top'], bool)

    def test_source_carpet_and_mirror_have_real_native_policies(self):
        profiles = {row['production_definition']['production_key']: row for row in json.loads(
            (ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())['records']}
        # Crystal carpets.lua:2-162; wall_mirror.lua:28 registers these real inputs.
        for item in (37365, 37366, 42340, 42341, 2603, 2976, 2975, 26076, 26077):
            profile = profiles[f'oteryn:item.tibia.i{item}']
            self.assertEqual(profile['production_binding']['disposition'], 'EXACT')
            self.assertTrue(profile['admission']['materializable'])
            self.assertIn('Ground', profile['admission']['legal_destinations'])
            self.assertIsInstance(profile['attributes']['always_on_top'], bool)
            self.assertIn(profile['attributes']['always_on_top_order'], (0, 1, 2, 3))

    def test_srx_updates_keep_source_top_order_after_specialized_overrides(self):
        facts = json.loads((ROOT / PROFILES).read_bytes())['always_on_top']
        records = json.loads((ROOT / 'content/items/definitions/srx1-item-updates.json').read_bytes())['records']
        for row in records:
            item = row['production_definition']['production_key'].removeprefix('oteryn:item.tibia.i')
            if item in facts:
                self.assertEqual(row['attributes']['always_on_top_order'], facts[item])
                self.assertEqual(row['attributes']['always_on_top'], facts[item] != 0)

    def test_top_order_enrichment_preserves_specialized_rules_and_unknowns(self):
        rows = [{'production_definition': {'production_key': 'oteryn:item.tibia.i1'},
                 'attributes': {'writing_policy': 'specialized'}, 'semantics': {'specialized': True}},
                {'production_definition': {'production_key': 'oteryn:item.tibia.i2'},
                 'attributes': {'writing_policy': 'specialized'}, 'semantics': {'specialized': True}}]
        before_unknown = copy.deepcopy(rows[1])
        apply_top_order(rows, {'always_on_top': {'1': 3}})
        self.assertEqual(rows[0]['attributes'], {'writing_policy': 'specialized',
                                               'always_on_top': True, 'always_on_top_order': 3})
        self.assertEqual(rows[0]['semantics'], {'specialized': True})
        self.assertEqual(rows[1], before_unknown)


if __name__ == '__main__':
    unittest.main()
