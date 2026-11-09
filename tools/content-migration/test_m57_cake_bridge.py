"""Owner-approved bridge identity must reach the actual canonical map registry."""
import json
import hashlib
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


class CakeBridgeIdentity(unittest.TestCase):
    def test_old_capture_digest_keeps_every_historical_binding_exact(self):
        import sys
        sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
        from build_spell_item_identities import historical_binding_bytes
        from drome_item_profiles import BINDINGS_SHA
        raw = (ROOT / 'imports/crystalserver/bindings/items.json').read_bytes()
        prior = historical_binding_bytes(raw, BINDINGS_SHA)
        self.assertEqual(hashlib.sha256(prior).hexdigest(), BINDINGS_SHA)
        for mutation in ('duplicate', 'old_binding', 'approved_binding'):
            changed = json.loads(raw)
            approved = next(r for r in changed['bindings'] if r['external_id'] == '48227')
            if mutation == 'duplicate':
                changed['bindings'].append(approved)
            elif mutation == 'old_binding':
                changed['bindings'][0]['source_revision'] = '0' * 40
            else:
                approved['target']['key'] = 'oteryn:item.tibia.i48228'
            with self.assertRaises(ValueError):
                historical_binding_bytes(json.dumps(changed).encode(), BINDINGS_SHA)

    def test_owner_approved_floor_has_exact_canonical_identity_and_physics(self):
        key = 'oteryn:item.tibia.i48227'
        reference = json.loads((ROOT / 'content/world/definitions/reference.json').read_bytes())
        rows = [r for r in reference['records'] if r['identity']['key'] == key]
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]['identity'], {
            'family': 'Item', 'key': key, 'revision': 'definition-r1'})
        bindings = json.loads((ROOT / 'imports/crystalserver/bindings/items.json').read_bytes())
        binding, = [b for b in bindings['bindings'] if b['external_id'] == '48227']
        self.assertEqual(binding['disposition'], 'EXACT')
        self.assertEqual(binding['target'], rows[0]['identity'])
        profiles = json.loads((ROOT / 'tools/content-schema/native-gameplay/event-item-profiles.json').read_bytes())
        self.assertNotIn(48227, [r['item_id'] for r in profiles['unavailable']])
        physical, = [r for r in profiles['world_map_physical'] if r['server_item_id'] == 48227]
        self.assertEqual((physical['ground'], physical['ground_speed'], physical['movable'],
                          physical['pickupable'], physical['block_solid']),
                         (True, 150, False, False, False))


if __name__ == '__main__':
    unittest.main()
