"""Registered profile fragments retain their identity and digest gates."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from questl2_native_fragments import item_profiles


class RegisteredNativeFragments(unittest.TestCase):
    def test_registered_login_kit_is_an_exact_native_profile(self):
        root = Path(__file__).resolve().parents[3]
        rows = item_profiles(root / 'content/items/definitions')
        selected = [r for r in rows if r['production_definition']['production_key'] ==
                    'oteryn:item.tibia.i3571']
        self.assertEqual(len(selected), 1)
        self.assertTrue(selected[0]['admission']['materializable'])

    def test_changed_bytes_and_parent_path_are_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            content = Path(directory)
            definitions = content / 'items/definitions'
            definitions.mkdir(parents=True)
            raw = b'{"records": []}\n'
            profile = definitions / 'native.json'
            profile.write_bytes(raw)
            manifest = {'item_profiles': {'path': 'items/definitions/native.json',
                                         'sha256': hashlib.sha256(raw).hexdigest()}}
            target = content / 'spells.manifest.json'
            target.write_text(json.dumps(manifest))
            self.assertEqual(item_profiles(definitions), [])
            profile.write_bytes(raw + b' ')
            with self.assertRaisesRegex(ValueError, 'digest mismatch'):
                item_profiles(definitions)
            manifest['item_profiles']['path'] = 'items/definitions/../native.json'
            target.write_text(json.dumps(manifest))
            with self.assertRaisesRegex(ValueError, 'outside registered'):
                item_profiles(definitions)


if __name__ == '__main__':
    unittest.main()
