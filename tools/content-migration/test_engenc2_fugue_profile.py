"""Fugue's pinned combat body must reach the production native provider."""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


class FugueProfileTests(unittest.TestCase):
    def test_inventory_extension_requires_the_entire_exact_source_body(self):
        import copy
        import engenc2_fugue_profile as profile
        source = json.loads((ROOT / profile.SOURCE).read_bytes())
        row, _ = profile.native(source)
        self.assertEqual(profile.inventory_extension(ROOT, [row]), (1, 4))
        with self.assertRaisesRegex(ValueError, 'FUGUE_INVENTORY_SOURCE_MISMATCH'):
            profile.inventory_extension(ROOT, [row, row])
        changed = copy.deepcopy(row)
        changed['profile']['data']['profile']['health'] += 1
        with self.assertRaisesRegex(ValueError, 'FUGUE_INVENTORY_SOURCE_MISMATCH'):
            profile.inventory_extension(ROOT, [changed])

    def test_production_fugue_retains_source_combat_and_no_foreign_attacks(self):
        document = json.loads((ROOT / 'content/creatures/definitions/spell-native-profiles.json').read_bytes())
        rows = [r for r in document['records'] if r['profile']['target']['key'] == 'oteryn:creature.fugue']
        self.assertEqual(len(rows), 1, 'Fugue has no unique production native combat body')
        row = rows[0]
        body = row['profile']['data']['profile']
        self.assertEqual((body['health'], body['experience'], body['speed'], body['armor'], body['details']['defense']), (17000, 10000, 210, 82, 50))
        behavior = row['behavior']['data']['profile']
        self.assertEqual([(a['interval_ms'], a['chance_ppm']) for a in behavior['attacks']], [(2000, 1000000), (2000, 200000), (1000, 140000)])
        self.assertEqual([(a['interval_ms'], a['chance_ppm']) for a in behavior['defenses']], [(2000, 200000)])
        formulas = [p['data']['profile'] for p in row['attack_definitions']['profiles'] if p['target']['family'] == 'Formula']
        self.assertEqual([(p['minimum'], p['maximum']) for p in formulas], [(150, 700), (200, 720), (500, 1000), (200, 650)])
        self.assertTrue(all('fugue' in p['target']['key'] for p in row['attack_definitions']['profiles']))
        self.assertEqual(row['monster_melee']['source_repository'], 'Meth28/global')
        self.assertEqual(row['monster_melee']['minimum'], 150)
        source = json.loads((ROOT / 'docs/reference/content-prep/prey-content-1/creature-source-profiles/fugue.json').read_bytes())
        self.assertEqual(row['monster_melee']['source_sha256'], source['sources'][0]['response_sha256'])
        self.assertIn('/blob/d959b2a99136978fc714622d42f6906b6d90e892/', source['sources'][0]['url'])


if __name__ == '__main__':
    unittest.main()
