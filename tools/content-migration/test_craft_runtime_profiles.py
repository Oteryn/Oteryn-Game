"""Cooking's native profile overlay preserves existing authority and rejects bad identities."""
import copy
import json
from pathlib import Path
import tempfile
import unittest

import craft_runtime_profiles as craft
import register_spell_families as registry

ROOT = Path(__file__).resolve().parents[2]


class CookingProfiles(unittest.TestCase):
    def test_overlay_admits_complete_recipe_closure_and_preserves_existing_profiles(self):
        generated = registry.outputs(ROOT)
        profile_path = registry.spell_import.PROVIDER_DESTINATIONS['item_profiles']
        records = json.loads(generated[profile_path])['records']
        keys = {row['production_definition']['production_key'] for row in records}
        self.assertTrue({'oteryn:item.tibia.i3604', 'oteryn:item.tibia.i2535',
                         'oteryn:item.tibia.i3603', 'oteryn:item.tibia.i1943'} <= keys)
        self.assertEqual(len(keys), len(records))
        self.assertEqual(generated[profile_path], (ROOT / profile_path).read_bytes())

    def test_existing_profile_identity_cannot_be_rebound(self):
        overlay = json.loads((ROOT / craft.PATH).read_bytes())
        existing = copy.deepcopy(overlay['records'][0])
        existing['production_binding']['target']['key'] = 'oteryn:item.tibia.i3600'
        generated = {'profiles': json.dumps({'records': [existing]}).encode(),
                     'manifest': b'{"item_profiles":{}}'}
        with self.assertRaisesRegex(ValueError, 'IDENTITY_CONFLICT'):
            craft.admit(ROOT, generated, 'profiles', 'manifest', registry.provider_order,
                        registry.spell_import.canonical_bytes)

    def test_missing_recipe_station_policy_fails_closed(self):
        with tempfile.TemporaryDirectory(prefix='.craft-profile-test-', dir=ROOT) as directory:
            root = Path(directory)
            (root / 'content/craft').mkdir(parents=True)
            overlay = json.loads((ROOT / craft.PATH).read_bytes())
            overlay['records'].pop()
            (root / craft.PATH).write_text(json.dumps(overlay))
            (root / 'content/craft/recipes.json').write_bytes(
                (ROOT / 'content/craft/recipes.json').read_bytes())
            with self.assertRaisesRegex(ValueError, 'RECORD_SET'):
                craft.admit(root, {}, 'profiles', 'manifest', registry.provider_order,
                            registry.spell_import.canonical_bytes)


if __name__ == '__main__':
    unittest.main()
