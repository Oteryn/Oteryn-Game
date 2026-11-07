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
    apply_carpet_types,
    initialized_item_ids,
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
        rotations = {(row['item_id'], row['target']) for row in catalogue['native_transforms'] if row['kind'] == 'ROTATE'}
        self.assertTrue({(35949,35969),(35969,35949),(35951,35971),(35971,35951)} <= rotations,
                        'both aqueduct steady phases retain their source rotations after decay')
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

    def test_native_xml_wrap_pairs_are_exact_forward_evidence(self):
        catalogue = json.loads((ROOT / CATALOGUE).read_bytes())
        profiles = json.loads((ROOT / PROFILES).read_bytes())
        keys = {row['production_definition']['production_key'] for row in profiles['records']}
        pairs = {(row['item_id'], row['target']) for row in catalogue['wrap_pairs']}
        self.assertEqual(len(pairs), 1617)
        self.assertTrue({(6355,6371),(34276,23398),(35949,23398),(35969,23398)} <= pairs)
        self.assertNotIn((23398,34276), pairs, 'reverse unwrap needs prior-wrap or purchased-origin provenance')
        for row in catalogue['wrap_pairs']:
            self.assertRegex(row['donor'], r'data/items/items.xml:\d+')
            self.assertIn(f"oteryn:item.tibia.i{row['item_id']}", keys)
            self.assertIn(f"oteryn:item.tibia.i{row['target']}", keys)
        self.assertIn(b'FUNCTION game_house_native_wrap_pair(p_source TEXT,p_target TEXT)', sql_pairs(catalogue, profiles))

    def test_initialized_appearance_requires_explicit_id_and_flags(self):
        # Crystal items.cpp:156-169,226 excludes missing flags, including an
        # object represented by the parser's default empty dictionary.
        self.assertEqual(initialized_item_ids(bytes.fromhex('0a0208010a0408021a000a021a00')), {2})

    def test_xml_only_carpets_are_qualified_without_fabricated_use_handlers(self):
        overlay = json.loads((ROOT / PROFILES).read_bytes())
        records = {row['production_definition']['production_key']: row for row in overlay['records']}
        catalogue = json.loads((ROOT / CATALOGUE).read_bytes())
        for item in (51222, 51223):
            row = records[f'oteryn:item.tibia.i{item}']
            self.assertEqual(row['production_binding']['disposition'], 'EXACT')
            self.assertTrue(row['admission']['materializable'])
            self.assertEqual(row['semantics']['classification']['value']['item_type'], {'state':'KNOWN','value':'CARPET'})
            self.assertNotIn(item, {action['item_id'] for action in catalogue['carpets']})
        self.assertEqual(sum(row['semantics']['classification'].get('value', {}).get('item_type') == {'state':'KNOWN','value':'CARPET'} for row in records.values()), 161)
        self.assertNotEqual(records['oteryn:item.tibia.i26151']['semantics']['classification']['value']['item_type'], {'state':'KNOWN','value':'CARPET'})
        existing = copy.deepcopy(records['oteryn:item.tibia.i37365'])
        existing['semantics']['classification']['value']['item_type'] = {'state':'NA'}
        existing['semantics']['classification']['value']['capabilities'] = {'state':'KNOWN','value':[]}
        apply_carpet_types([existing], overlay)
        self.assertEqual(existing['semantics']['classification']['value']['item_type'], {'state':'KNOWN','value':'CARPET'})
        self.assertEqual(existing['semantics']['classification']['value']['capabilities'], {'state':'KNOWN','value':[]})

    def test_native_decoration_temporal_targets_are_closed(self):
        profiles = {row['production_definition']['production_key']: row for row in json.loads(
            (ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())['records']}
        for source, target in ((35949, 35950), (35969, 35970)):
            row = profiles[f'oteryn:item.tibia.i{source}']
            temporal = row['semantics']['temporal']['value']
            self.assertEqual(temporal['decay_target']['value']['key'], f'oteryn:item.tibia.i{target}')
            self.assertTrue(f'oteryn:item.tibia.i{target}' in profiles,
                            f'native decay target {target} must be admitted')

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
