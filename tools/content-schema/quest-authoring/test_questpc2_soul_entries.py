"""Named Soul War entry facts retain source slot order and Lua last assignment."""
import json
import unittest
from pathlib import Path
import questpc2_soul_entries as tool


class SoulEntryFactsTests(unittest.TestCase):
    def test_named_configs_keep_all_slots_and_last_cooldown(self):
        root = Path(__file__).resolve().parents[3]
        facts = tool.capture(root)
        rows = {row['name']: row for row in facts['entries']}
        self.assertEqual(len(rows), 6)
        greed = rows['goshnarsGreed']
        self.assertEqual(greed['cooldown_assignments'], [0, 72000])
        self.assertEqual(greed['cooldown_seconds'], 72000)
        self.assertEqual(greed['lever'], [33775, 31665, 14])
        self.assertEqual(greed['zone_from'], [33737, 31658, 14])
        self.assertEqual(greed['zone_to'], [33755, 31673, 14])
        for row in rows.values():
            self.assertEqual(len(row['players']), 5)
            self.assertEqual(len({tuple(p['from']) for p in row['players']}), 5)
            self.assertEqual(len({tuple(p['to']) for p in row['players']}), 1)
            self.assertEqual(row['minimum_level'], 250)
            self.assertEqual(row['exit'], [33621, 31427, 10])
        self.assertEqual(rows['goshnarsMegalomania']['cooldown_seconds'], 259200)
        self.assertEqual(rows['goshnarsMegalomania']['boss'], [33710, 31634, 14])
        self.assertFalse(facts['native_admission'])
        self.assertEqual(facts['source']['sha256'],
                         '89c0024914b0593b9b7d2bfb535a24025a552f54b158319697cdf56a66375e75')
        self.assertEqual(facts, json.loads((root / tool.OUTPUT).read_text()))
