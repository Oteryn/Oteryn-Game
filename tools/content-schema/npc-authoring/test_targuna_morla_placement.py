"""Morla uses the retained Global candidate cell, without enabling other Targuna stages."""
import json
import unittest
import convert_placements as producer


class MorlaPlacement(unittest.TestCase):
    def test_exact_morla_placement_retains_source_evidence(self):
        files = producer.convert()
        rows = [r['declaration'] for name, text in files.items() if name.startswith('placements-')
                for r in json.loads(text)['records'] if r['declaration']['npc'] == 'oteryn:npc.morla']
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]['cell'], dict(x=33514, y=32748, floor=8))
        self.assertEqual(rows[0]['direction'], 'north')
        self.assertTrue(any(p['origin'] == 'crystal' and p['definition_sha256'] ==
            '6d4996501d42f7750acf94e99f8e9b6fc36e65d88d77ae190f5d73875b5b62ec'
            for p in rows[0]['provenance']))


if __name__ == '__main__':
    unittest.main()
