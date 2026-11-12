"""Six retained Rotten Blood Item facts keep source identity and loader defaults."""
import json
import tempfile
import unittest
from pathlib import Path
import questpc2_rotten_item_facts as tool


class RottenItemFactsTests(unittest.TestCase):
    def test_archive_tampering_cannot_supply_source_facts(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            directory = root / tool.DIRECTORY
            directory.mkdir(parents=True)
            (directory / 'changed.tar.gz').write_bytes(b'not the retained source')
            (directory / 'corpus-input.json').write_text(json.dumps({
                'path': 'changed.tar.gz', 'sha256': '0' * 64,
            }))
            with self.assertRaisesRegex(ValueError, 'donor archive pin differs'):
                tool.capture(root)

    def test_exact_six_source_items_are_candidates_without_runtime_admission(self):
        root = Path(__file__).resolve().parents[3]
        facts = tool.capture(root)
        self.assertFalse(facts['native_admission'])
        self.assertFalse(facts['use_effects_admitted'])
        self.assertEqual(facts['source']['revision'], '00ce02a57ca5a12e48f32a3476e37471167e4c3f')
        self.assertEqual(facts['source']['sha256'],
                         '13a8773e34085daad1a716465c0510060d1f2255c4bc69995fd160c8b4afcece')
        expected = {
            43501: ('the essence of Murcion', 75234, True),
            43502: ('the essence of Ichgahal', 75237, True),
            43503: ('the essence of Vemiath', 75240, True),
            43504: ('the essence of Chagorz', 75243, True),
            43899: ('cursed wood', 76732, False),
            43900: ('darklight geode', 76736, False),
        }
        self.assertEqual({r['source_item_id'] for r in facts['items']}, set(expected))
        for row in facts['items']:
            name, line, stackable = expected[row['source_item_id']]
            self.assertEqual((row['name'], row['source_line']), (name, line))
            self.assertEqual(row['weight_evidence'], 'loader_default_zero')
            self.assertEqual(row['semantics']['physical']['value'], {
                'weight': {'state': 'KNOWN', 'value': 0},
                'movable': {'state': 'KNOWN', 'value': True},
                'pickupable': {'state': 'KNOWN', 'value': True},
            })
            self.assertEqual(row['semantics']['stack']['value']['stackable'],
                             {'state': 'KNOWN', 'value': stackable})
            self.assertEqual(row['candidate_admission']['legal_destinations'],
                             ['CharacterInventory', 'Ground'])
            for group in ('container', 'charges', 'temporal'):
                self.assertEqual(row['semantics'][group], {'state': 'NOT_APPLICABLE'})
        self.assertEqual(facts, json.loads((root / tool.OUTPUT).read_text()))
