"""Every source-resolvable taming rule must reach the production cosmetics input."""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


class TamingCatalogue(unittest.TestCase):
    def test_source_resolvable_rules_are_admitted_with_exact_grants(self):
        document = json.loads((ROOT / 'content/cosmetics/outfits/authoring.json').read_bytes())['document']
        mounts = {row['key'] for row in document['mounts']}
        grants = {row['grant_key']: row for row in document['grant_mappings']}
        rules = json.loads((ROOT / 'content/cosmetics/taming.json').read_bytes())
        for rule in rules:
            with self.subTest(mount=rule['mount_key'], item=rule['item_key']):
                self.assertIn(rule['mount_key'], mounts)
                self.assertEqual(grants[rule['grant_key']]['target'],
                                 {'kind': 'Mount', 'mount_key': rule['mount_key']})
                self.assertEqual(grants[rule['grant_key']]['compatible_provenance'], ['mount-taming-v1'])


if __name__ == '__main__':
    unittest.main()
