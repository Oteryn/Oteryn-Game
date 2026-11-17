"""Chosen collector cells retain source positions and cannot bypass native obstacles."""
import unittest
from pathlib import Path
import chosen_placement_adjustments as source

ROOT = Path(__file__).resolve().parents[3]


class CollectorCells(unittest.TestCase):
    def test_adjacent_edala_cell_records_native_evidence(self):
        choice = {'source': {'x':32697,'y':31718,'z':2},
                  'target': {'x':32698,'y':31718,'z':2},
                  'reason':'Original source cell is BlockSolid; keep the contact on adjacent native ground.'}
        cell, evidence = source.choose_cell(ROOT, (32697,31718,2), choice, set())
        self.assertEqual(cell, (32698,31718,2))
        self.assertEqual(evidence['basis'], 'CHOSEN_OTERYN_APPROXIMATION')
        self.assertEqual(evidence['source_cell'], {'x':32697,'y':31718,'floor':2})
        self.assertEqual(len(evidence['map']['sha256']), 64)
        for bad in [dict(choice, target={'x':32700,'y':31718,'z':2}),
                    dict(choice, target={'x':32698,'y':31718,'z':3}),
                    dict(choice, source={'x':32696,'y':31718,'z':2})]:
            with self.assertRaises(ValueError):
                source.choose_cell(ROOT, (32697,31718,2), bad, set())
        with self.assertRaises(ValueError):
            source.choose_cell(ROOT, (32697,31718,2), choice, {cell})

    def test_humphrey_cannot_move_to_another_altar(self):
        choice = {'source':{'x':32358,'y':31687,'z':6},
                  'target':{'x':32359,'y':31687,'z':6},'reason':'Blocked adjacent cell must refuse.'}
        with self.assertRaisesRegex(ValueError, 'clear native ground'):
            source.choose_cell(ROOT, (32358,31687,6), choice, set())


if __name__ == '__main__':
    unittest.main()
