"""Current retail easels retain the exact Wiki article identity when rotated."""
from pathlib import Path
import json
import unittest

ROOT = Path(__file__).resolve().parents[2]

class EaselRotation(unittest.TestCase):
    def test_each_current_wiki_identity_rotates_only_between_its_two_states(self):
        evidence = json.loads((ROOT / 'docs/architecture/reviews/SRV7-W18-ARTIST-EASEL-SOURCE-2026-11-06.json').read_text())
        rules = json.loads((ROOT / 'tools/content-schema/native-gameplay/furniture-rules.json').read_text())
        actual = dict(rules['rotate'])
        for article in evidence['states']:
            endpoints = [row[0] for row in article['ids']]
            self.assertEqual(len(endpoints), 2, article['title'])
            for before, after in article['qualified_rotations']:
                self.assertEqual(actual.get(before), after, article['title'])
                self.assertEqual(actual.get(after), before, article['title'])

if __name__ == '__main__':
    unittest.main()
