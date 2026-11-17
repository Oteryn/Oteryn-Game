"""Every QREADY1 reward must be admitted by the actual native Item provider."""
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools/content-migration'))
sys.path.insert(0, str(ROOT / 'tools/content-schema/world-authoring'))
import world_region_codec as codec
import qready1_reward_profiles as profiles
import register_spell_families as registry


class Qready1NativeRewardTest(unittest.TestCase):
    def test_all_owned_claim_rewards_have_native_inventory_profiles(self):
        native = json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())
        by_key = {r['production_definition']['production_key']: r for r in native['records']}
        capture = json.loads((ROOT / 'tools/content-schema/world-authoring/samples/quest-chest-bindings/qready1.json').read_bytes())
        selected = {row['claim'] for row in capture['bindings']}
        seen = set()
        for path in (ROOT / 'content/interactions/reward_claims').glob('reward-claim*.json'):
            for row in json.loads(path.read_bytes())['records']:
                claim = row['definition']
                if claim['identity']['key'] not in selected:
                    continue
                for placement in claim['placements']:
                    for item in placement['reward']['items']:
                        record = by_key[item['item']['key']]
                        self.assertTrue(record['admission']['materializable'])
                        self.assertIn('CharacterInventory', record['admission']['legal_destinations'])
                        self.assertIn(record['admission']['stack_class'], ('NonStackable', 'StackCapable'))
                seen.add(claim['identity']['key'])
        self.assertEqual(selected, seen)

    def test_reused_profiles_requalify_without_changing_existing_native_facts(self):
        path = 'content/items/definitions/spell-native-profiles.json'
        generated = {p: (ROOT / p).read_bytes() for p in (path, registry.NATIVE_MANIFEST)}
        before = json.loads(generated[path])
        profiles.admit(ROOT, generated, path, registry.NATIVE_MANIFEST,
                       registry.provider_order, registry.spell_import.canonical_bytes)
        self.assertEqual(before, json.loads(generated[path]))

    def test_shipped_world_has_every_exact_owned_claim_binding(self):
        index = json.loads((ROOT / 'content/world/placements/index.json').read_bytes())
        capture = json.loads((ROOT / 'tools/content-schema/world-authoring/samples/quest-chest-bindings/qready1.json').read_bytes())
        regions = {}
        for row in capture['bindings']:
            x, y, z = row['position']
            path = 'content/world/placements/' + codec.region_name(z, x // 256, y // 256)
            if path not in regions:
                regions[path] = {(t[0], t[1]): t for _, tiles in codec.decode_region((ROOT / path).read_bytes())[3] for t in tiles}
            tile = regions[path][(x, y)]
            self.assertEqual(0, tile[3], row['claim'])
            matches = [(palette, depth, attrs) for palette, depth, attrs in tile[5]
                       if depth == 0 and (attrs or {}).get('unique') == row['unique']
                       and index['palette'][palette]['source_item_id'] == row['appearance']]
            self.assertEqual(1, len(matches), row['claim'])
