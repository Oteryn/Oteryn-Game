"""Exact native Fury entry geometry must retain its source owner and stack."""
import copy
import json
from pathlib import Path
import unittest
from unittest import mock
import questpc_explore_bindings as explore

import questpc_fury_approaches as fury

ROOT = Path(__file__).resolve().parents[3]


class FuryApproachTests(unittest.TestCase):
    def test_thais_entry_uses_the_retained_ground_outside_the_portal(self):
        at, appearance, sources, owner = fury.resolve(ROOT, fury.QUEST, 'fury-approach:thais')
        self.assertEqual(at, dict(x=32264, y=32165, floor=-7))
        self.assertEqual(appearance, 7066)
        self.assertEqual(owner, 'fury_gates/thais')
        self.assertTrue(any('201189f871d23597b82caf88fd611454a2ddefb8' in s for s in sources))

    def test_foreign_quest_or_unreviewed_city_cannot_borrow_entry(self):
        for quest, reference in [('oteryn:quest.longsword_quest', 'fury-approach:thais'),
                                 (fury.QUEST, 'fury-approach:venore')]:
            with self.assertRaises(ValueError):
                fury.resolve(ROOT, quest, reference)

    def test_source_revision_or_portal_stack_change_fails_closed(self):
        original = json.loads((ROOT / fury.SOURCE).read_text())
        for field in ('revision', 'portal'):
            data = copy.deepcopy(original)
            if field == 'revision':
                data['source_revision'] = 'changed'
            else:
                layer = next(l for l in data['layers'] if l['key'] == 'fury_gates/thais')
                next(t for t in layer['tiles'] if t['position'] == [32264,32164,7])['entries'][1][1] = 9711
            with self.assertRaises(ValueError):
                fury.resolve(ROOT, fury.QUEST, 'fury-approach:thais', layers=data)

    def test_reviewed_fury_callback_preserves_level_premium_and_exact_world_city(self):
        binding = ((fury.QUEST, 's1', 'fury-approach:thais',
                    'chosen-source/fury_gates_mini_world_change'),)
        with mock.patch.object(explore, 'BINDINGS', binding):
            callback, = explore.callbacks(ROOT)
        self.assertEqual(callback['world_change'], 'fury_gates/thais')
        self.assertEqual(callback['minimum_level'], 60)
        self.assertEqual(callback['admission'], {'premium': True})
        self.assertEqual(callback['selector']['native'], dict(
            position=dict(x=32264,y=32165,floor=-7), ordinal=0, appearance=7066))
