"""M41 only admits the pinned, exact Alyxo branch; it does not mark a whole quest done."""
import json
import unittest

import m41_npc_sources as tool


class M41NpcSourceTests(unittest.TestCase):
    def test_committed_finale_is_generator_output(self):
        self.assertEqual(json.loads((tool.ROOT / tool.PATH).read_bytes()), tool.build())

    def test_finale_keeps_topic_prerequisite_and_all_atomic_effects(self):
        packet = tool.build()
        self.assertEqual(packet['source_files'][0]['sha256'], tool.SHA)
        offer, finale = packet['rules']
        self.assertEqual(offer['effects'], [{'kind': 'set_topic', 'value': 13}])
        self.assertEqual(finale['topics'], [13])
        self.assertEqual(finale['guards'], [{'kind': 'storage', 'track': tool.FAFNAR, 'op': 'eq', 'value': 301}])
        self.assertEqual(finale['effects'], [
            {'kind': 'achievement', 'name': 'Sculptor Apprentice'},
            {'kind': 'give_item', 'item': 'oteryn:item.tibia.i31574', 'count': 1},
            {'kind': 'set', 'track': tool.REMAINS, 'value': 1},
            {'kind': 'set_topic', 'value': 14},
        ])
        self.assertFalse(any('completes' in rule for rule in packet['rules']))
