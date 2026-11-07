"""Pinned S06 missing-operand and reciprocal wedding-cost physics coverage."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    'srv7_operand_profiles', ROOT / 'tools/content-migration/acquisition_loot_item_profiles.py'
)
OWNER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(OWNER)
# The source-program operand census found these thirty missing profiles.9586 is
# the additional literal Lynda wedding-box cost;9056 retains the Hellspawn body.
EXPECTED = {9056, 9586, 10198, 10199, 11700, 11701, 14021, 14022, 14023,
            20275, 20276, 22721, 28792, 28793, 30168, 30169, 3348, 3403,
            5886, 5903, 5921, 5945, 5947, 6099, 6100, 6101, 6102, 6107,
            7289, 894, 895, 9041}


class CosmeticOperandProfilesTest(unittest.TestCase):
    def test_source_capture_covers_missing_npc_operands_and_wedding_cost(self):
        self.assertEqual(OWNER.IDS, EXPECTED)
        overlay = json.loads((ROOT / OWNER.PROFILE).read_bytes())
        actual = {int(row['production_definition']['production_key'].rsplit('i', 1)[1])
                  for row in overlay['records']}
        self.assertEqual(actual, EXPECTED)
        for row in overlay['records']:
            if row['production_definition']['production_key'] != 'oteryn:item.tibia.i9056':
                self.assertTrue(row['admission']['materializable'])
                self.assertIn('CharacterInventory', row['admission']['legal_destinations'])

    def test_owner_rejects_same_count_unqualified_source_substitution(self):
        overlay = json.loads((ROOT / OWNER.PROFILE).read_bytes())
        overlay = copy.deepcopy(overlay)
        overlay['records'][0]['production_definition']['production_key'] = 'oteryn:item.tibia.i999999'
        class AlteredPath:
            def read_bytes(self):
                return json.dumps(overlay).encode()
        class AlteredRoot:
            def __truediv__(self, _):
                return AlteredPath()
        with self.assertRaisesRegex(ValueError, 'PROFILE_COVERAGE'):
            OWNER.admit(AlteredRoot(), {}, 'unused', 'unused', lambda row: '', lambda value: b'')
