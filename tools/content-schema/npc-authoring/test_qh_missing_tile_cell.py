"""A source cell with no native tile moves only to the nearest walkable ground of its floor."""
import unittest
from pathlib import Path
import chosen_placement_adjustments as source

ROOT = Path(__file__).resolve().parents[3]
JIMMY = (32857, 31383, 6)


def choice(x, y, z=6):
    return {'source': {'x': JIMMY[0], 'y': JIMMY[1], 'z': JIMMY[2]},
            'target': {'x': x, 'y': y, 'z': z},
            'reason': 'Wiki cell has no native tile; nearest walkable ground in the same room.'}


class MissingTileCell(unittest.TestCase):
    def test_jimmy_moves_to_nearest_walkable_room_cell(self):
        cell, evidence = source.choose_cell(ROOT, JIMMY, choice(32857, 31380), set())
        self.assertEqual(cell, (32857, 31380, 6))
        self.assertEqual(evidence['source_cell'], {'x': 32857, 'y': 31383, 'floor': 6})
        self.assertEqual(len(evidence['map']['sha256']), 64)

    def test_farther_border_occupied_or_other_floor_cells_refuse(self):
        for x, y, z in [(32858, 31380, 6),   # walkable, but farther than 32857,31380
                        (32857, 31381, 6),   # nearer, but a roof border, not ground
                        (32856, 31380, 6),   # border
                        (32857, 31379, 6),   # farther
                        (32857, 31383, 7)]:  # another floor
            with self.assertRaises(ValueError):
                source.choose_cell(ROOT, JIMMY, choice(x, y, z), set())
        with self.assertRaises(ValueError):
            source.choose_cell(ROOT, JIMMY, choice(32857, 31380), {(32857, 31380, 6)})

    def test_existing_source_tile_still_needs_an_adjacent_target(self):
        edala = {'source': {'x': 32697, 'y': 31718, 'z': 2},
                 'target': {'x': 32699, 'y': 31718, 'z': 2}, 'reason': 'two cells away'}
        with self.assertRaises(ValueError):
            source.choose_cell(ROOT, (32697, 31718, 2), edala, set())


if __name__ == '__main__':
    unittest.main()
