"""NX-5 source-backed production catalogue, distinct from the synthetic format fixture."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[3]


class ProductionCatalogue(unittest.TestCase):
    def test_every_global_item_grant_has_a_canonical_target(self):
        author = json.loads((ROOT / 'content/cosmetics/outfits/authoring.json').read_bytes())
        actions = json.loads((ROOT / 'tools/content-schema/native-gameplay/item-cosmetic-actions.json').read_bytes())
        ordinary = [r for r in actions['rules'] if r['mode'] not in ('random_outfit', 'random_mount')]
        expected = {key for rule in ordinary for choice in rule['choices']
                    for field in ('grants', 'required', 'complete_grants') for key in choice.get(field, [])}
        all_mappings = author['document']['grant_mappings']
        baseline = expected | {
            'oteryn:appearance-grant/quest/soul-war/revenant-base',
            'oteryn:appearance-grant/quest/rotten-blood/decaying-defender-base',
        }
        mappings = [m for m in all_mappings if m['grant_key'] in baseline]
        self.assertEqual({m['grant_key'] for m in mappings}, baseline)
        self.assertEqual(len(ordinary), 46)
        self.assertEqual(len(mappings), 49)
        canonical = [o for o in author['document']['outfits']
                     if o['key'].startswith('oteryn:content.outfit.')]
        legacy = [o for o in author['document']['outfits']
                  if o['key'].startswith('oteryn:outfit.')]
        self.assertEqual(len(canonical), 14)
        self.assertEqual(len(legacy), 63)
        self.assertEqual(len(author['document']['outfits']),77)
        quest_keys = {f"{o['key']}/quest-grant/{addon}" for o in legacy
                      for addon in range(o['addon_count']+1)}
        self.assertEqual({m['grant_key'] for m in all_mappings}, baseline | quest_keys)
        self.assertEqual(len(all_mappings),len(baseline)+len(quest_keys))
        aliases = author['document']['outfit_aliases']
        self.assertEqual(len(aliases),14)
        self.assertEqual(set(aliases.values()),{o['key'] for o in canonical})
        rows={o['key']:o for o in canonical+legacy}
        for old,target in aliases.items():
            for field in ('female','male','addon_count','starter_female','starter_male'):
                self.assertEqual(rows[old][field],rows[target][field])
            self.assertFalse(rows[old]['premium'] and not rows[target]['premium'])
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
        for row in canonical:
            self.assertEqual(row['starter_male'], row['key'].endswith('.citizen_outfits'))
            self.assertEqual(row['starter_female'], row['starter_male'])
            self.assertEqual(row['starter_addons_male'] | row['starter_addons_female'], 0)


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
