"""The source-owned boss policy bodies must survive canonical House enrichment."""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


class CompleteBossPolicyTests(unittest.TestCase):
    def test_complete_boss_policies_match_the_actual_native_catalogue(self):
        native = {}
        for path in ['content/items/definitions/spell-native-profiles.json',
                     'content/items/definitions/srx1-item-updates.json']:
            for row in json.loads((ROOT / path).read_bytes())['records']:
                native[row['production_binding']['external_id']] = row
        cohorts = [
            ('boss-reward-yield-profiles.json',
             {'43778', '43779', '43780', '43781', '43782', '43860', '43946', '43947', '44048'}),
            ('boss-movement-item-profiles.json', None),
            ('boss-corpse-item-profiles.json', {'43666'}),
        ]
        count = 0
        for filename, selected in cohorts:
            capture = json.loads((ROOT / 'tools/content-schema/native-gameplay' / filename).read_bytes())
            self.assertEqual(capture['source_revision'], '201189f871d23597b82caf88fd611454a2ddefb8')
            for expected in capture['records']:
                key = expected['production_binding']['external_id']
                if selected is not None and key not in selected:
                    continue
                with self.subTest(item=key):
                    self.assertEqual(native[key], expected)
                count += 1
        self.assertEqual(count, 23)


if __name__ == '__main__':
    unittest.main()
