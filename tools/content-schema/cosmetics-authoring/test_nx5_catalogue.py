"""NX-5 source-backed production catalogue, distinct from the synthetic format fixture."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[3]


class ProductionCatalogue(unittest.TestCase):
    def test_every_global_item_grant_has_a_canonical_target(self):
        author = json.loads((ROOT / 'tools/content-schema/cosmetics-authoring/nx5-authoring.json').read_bytes())
        actions = json.loads((ROOT / 'tools/content-schema/native-gameplay/item-cosmetic-actions.json').read_bytes())
        ordinary = [r for r in actions['rules'] if r['mode'] not in ('random_outfit', 'random_mount')]
        expected = {key for rule in ordinary for choice in rule['choices']
                    for field in ('grants', 'required', 'complete_grants') for key in choice.get(field, [])}
        mappings = author['document']['grant_mappings']
        self.assertEqual({m['grant_key'] for m in mappings}, expected | {
            'oteryn:appearance-grant/quest/soul-war/revenant-base',
            'oteryn:appearance-grant/quest/rotten-blood/decaying-defender-base',
        })
        self.assertEqual(len(ordinary), 46)
        self.assertEqual(len(mappings), 49)
        self.assertEqual(len(author['document']['outfits']), 14)
        self.assertEqual(len(author['document']['mounts']), 8)
        # Mount targets retain the identities already declared by content.
        for family in ('mounts',):
            declared = set()
            for shard in (ROOT / 'content/cosmetics' / family).glob(f'{family}-*.json'):
                rows = json.loads(shard.read_bytes())
                for row in rows['records']:
                    declared.add(row['declaration']['identity']['key'])
            self.assertTrue({row['key'] for row in author['document'][family]} <= declared)
        self.assertEqual(len(set(map(tuple, author['document']['palette']['colours']))), 133)
        for row in author['document']['outfits']:
            self.assertEqual(row['starter_male'], row['key'].endswith('.citizen_outfits'))
            self.assertEqual(row['starter_female'], row['starter_male'])
            self.assertEqual(row['starter_addons_male'] | row['starter_addons_female'], 0)


    def test_combined_catalogue_preserves_nx5_identities_and_grant_effects(self):
        source = json.loads((ROOT / 'tools/content-schema/cosmetics-authoring/nx5-authoring.json').read_bytes())['document']
        combined = json.loads((ROOT / 'content/cosmetics/outfits/authoring.json').read_bytes())['document']
        self.assertEqual(combined['palette']['colours'], source['palette']['colours'])
        for family in ('outfits', 'mounts'):
            by_key = {row['key']: row for row in combined[family]}
            for row in source[family]:
                self.assertEqual(by_key[row['key']], row)
        outfits = {row['key']: row for row in combined['outfits']}
        grants = {row['grant_key']: row for row in combined['grant_mappings']}
        for row in source['grant_mappings']:
            actual = grants[row['grant_key']]
            self.assertEqual(actual['compatible_provenance'], row['compatible_provenance'])
            self.assertEqual(actual['target']['kind'], row['target']['kind'])
            if row['target']['kind'] == 'Mount':
                self.assertEqual(actual['target'], row['target'])
            else:
                if actual['target'] != row['target']:
                    self.assertIn(row['target']['outfit_key'], actual['outfit_aliases'])
                expected_outfit = outfits[row['target']['outfit_key']]
                actual_outfit = outfits[actual['target']['outfit_key']]
                for sex in ('female', 'male'):
                    self.assertEqual(actual_outfit[sex], expected_outfit[sex])
                self.assertEqual(actual['target'].get('addon'), row['target'].get('addon'))


class BoundedRuntimeInput(unittest.TestCase):
    def test_native_cosmetics_descriptor_is_bounded_and_does_not_embed_source_shards(self):
        import importlib.util
        tool = ROOT / 'tools/content-schema/cosmetics-authoring/cosmetics_authoring.py'
        spec = importlib.util.spec_from_file_location('nx5_cosmetics', tool)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        source = ROOT / module.AUTHORING
        original = source.read_bytes()
        data = module.materialized_outputs(ROOT)[module.OUTPUT]
        self.assertEqual(source.read_bytes(), original, 'maintained authoring is never generated')
        self.assertLess(len(data), 200000)
        descriptor = json.loads(data)
        self.assertEqual(descriptor['schema'], 'OTERYN_NATIVE_COSMETICS_INPUT/v1')
        self.assertNotIn('source_shards', descriptor)


if __name__ == '__main__':
    unittest.main()
