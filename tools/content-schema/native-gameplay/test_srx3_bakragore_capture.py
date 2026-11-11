"""Literal source essence coordinates must exist in the native map capture."""
import json
from pathlib import Path
import unittest

ROOT=Path(__file__).resolve().parents[3]
POINTS={(33085,32398,15),(33081,32398,15),(33076,32389,15),(33076,32395,15),
        (33067,32398,15),(33071,32398,15),(33076,32407,15),(33076,32401,15)}
IDS={43497,43498,43507,43508,43983,43984,43985,43986,43987,43988}


class BakragoreCaptureTests(unittest.TestCase):
    def test_actual_source_capture_contains_literal_essence_targets(self):
        capture=json.loads((ROOT/'content/encounters/rotten-blood-bakragore/source-room.json').read_bytes())
        actual={tuple(row['source_position']) for row in capture['tiles']}
        self.assertFalse(POINTS-actual, f'uncaptured literal donor essence tiles: {sorted(POINTS-actual)}')
        self.assertLessEqual(IDS,{row['server_item_id'] for row in capture['item_policies']})


if __name__=='__main__':unittest.main()
