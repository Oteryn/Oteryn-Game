"""Compiled native annotations never waive the frozen SOURCE core or native hold."""
import copy
import unittest
from pathlib import Path

import quest_tree_authoring as tool


class MonkNativeLoweringTests(unittest.TestCase):
    def setUp(self):
        self.root = Path(__file__).resolve().parents[3]
        self.row = {'definition': {
            'identity': {'key': 'oteryn:quest.the_way_of_the_monk_quest'},
            'native_lowering': tool.monk_native_lowering(self.root),
            'source_data': {'unchanged': True}, 'readiness': 'waiting_data',
        }}

    def test_native_projection_preserves_source_fields_and_original_hold(self):
        before = copy.deepcopy(self.row)
        result = tool.source_projection(self.root, [self.row])[0]['definition']
        self.assertEqual(self.row, before)
        self.assertEqual(result['source_data'], before['definition']['source_data'])
        self.assertEqual(result['readiness'], 'waiting_data')
        self.assertEqual(result['native_lowering']['state'], 'WAITING_IMPLEMENTATION')
        self.assertEqual(result['native_lowering']['canonical_progress_refs'], [])
        self.assertEqual(len(self.row['definition']['native_lowering']['canonical_interaction_refs']), 13)

    def test_uncompiled_reference_or_false_readiness_cannot_be_projected_away(self):
        for field, value in [
            ('state', 'WAITING_IMPLEMENTATION'), ('canonical_progress_refs', ['oteryn:quest-progress/invented']),
            ('canonical_interaction_refs', []), ('reason', 'unreviewed'),
        ]:
            with self.subTest(field=field):
                row = copy.deepcopy(self.row)
                row['definition']['native_lowering'][field] = value
                with self.assertRaisesRegex(ValueError, 'native annotation'):
                    tool.source_projection(self.root, [row])
