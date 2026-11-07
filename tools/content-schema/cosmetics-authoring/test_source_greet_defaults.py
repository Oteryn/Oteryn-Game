"""Pinned source-initialization capture guards against nested-setter regex leaks."""
import json
from pathlib import Path
import unittest

ROOT=Path(__file__).resolve().parents[3]

class SourceGreetDefaults(unittest.TestCase):
    def test_registered_source_defaults_match_qualified_initialization(self):
        capture=json.loads((ROOT/'tools/content-schema/cosmetics-authoring/source-default-greetings.json').read_bytes())
        programme=json.loads((ROOT/'content/cosmetics/npc-acquisitions.json').read_bytes())
        nodes={row['npc']:row for row in programme['npcs']}
        self.assertEqual(capture['schema'],'OTERYN_SOURCE_TOPLEVEL_GREET_CAPTURE/v1')
        self.assertEqual(capture['source_revision'],programme['revision'])
        self.assertEqual(len(capture['npcs']),12)
        for row in capture['npcs']:
            with self.subTest(npc=row['npc']):
                actual=nodes[row['npc']]
                self.assertEqual(actual['source']['sha256'],row['source_sha256'])
                self.assertEqual(actual['source']['path'],row['source_path'])
                expected="Whatcha do in my place?" if row['npc']=='oteryn:npc.ajax' else row['default_greeting']
                self.assertEqual(actual['greeting'],expected)

if __name__=='__main__':unittest.main()
