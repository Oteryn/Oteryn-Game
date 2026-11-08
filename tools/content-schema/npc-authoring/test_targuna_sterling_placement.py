"""Sterling uses the retained Global candidate cell, without enabling other Targuna stages."""
import json
import unittest
import convert_placements as producer


class SterlingPlacement(unittest.TestCase):
    def test_exact_sterling_placement_retains_source_evidence(self):
        files = producer.convert()
        rows = [r['declaration'] for name, text in files.items() if name.startswith('placements-')
                for r in json.loads(text)['records'] if r['declaration']['npc'] == 'oteryn:npc.sterling']
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]['cell'], dict(x=31928, y=31903, floor=7))
        self.assertEqual(rows[0]['direction'], 'north')
        self.assertTrue(any(p['origin'] == 'crystal' and p['definition_sha256'] ==
            '3069d6f8265d9b414aedc6c50460268d5c3e98c0a65257cc48c54750e44516a9'
            for p in rows[0]['provenance']))


if __name__ == '__main__':
    unittest.main()
