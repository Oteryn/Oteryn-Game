"""Retained Summer membership evidence must preserve admitted identities."""
import json
from pathlib import Path
import unittest

import questpc2_rotten_membership_facts as tool


class RottenMembershipFactsTests(unittest.TestCase):
    def test_six_memberships_keep_exact_bindings_without_runtime_admission(self):
        root = Path(__file__).resolve().parents[3]
        facts = tool.capture(root)
        self.assertFalse(facts['native_admission'])
        self.assertFalse(facts['use_effects_admitted'])
        expected = {
            43961: ('darklight figurine', 76781),
            43962: ('putrefactive figurine', 76785),
            43964: ("Ichgahal's fungal infestation", 76793),
            43966: ('Chagorz igneous obsidian', 76801),
            43967: ("Vemiath's infused basalt", 76805),
            43968: ("Bakragore's amalgamation", 76809),
        }
        self.assertEqual({r['source_item_id'] for r in facts['items']}, set(expected))
        self.assertEqual(facts['source']['sha256'], tool.previous.XML_SHA)
        self.assertEqual(facts['source']['path'], 'data/items/items.xml')
        bindings = json.loads((root / tool.shared.BINDINGS).read_bytes())['bindings']
        for row in facts['items']:
            binding = row['identity_binding']
            self.assertIn(binding, bindings)
            self.assertEqual(binding['source_revision'], tool.previous.REVISION)
            self.assertEqual(binding['external_id'], str(row['source_item_id']))
            self.assertEqual(binding['disposition'], 'EXACT')
            self.assertEqual(binding['target'], {
                'family': 'Item', 'key': f"oteryn:item.tibia.i{row['source_item_id']}",
                'revision': 'definition-r1',
            })
            self.assertEqual((row['name'], row['source_line']), expected[row['source_item_id']])
        self.assertEqual(facts, json.loads((root / tool.OUTPUT).read_bytes()))
