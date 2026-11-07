"""Reachable Halloween food uses the retained current-Tibia food admission."""
import json
from pathlib import Path
import unittest
ROOT = Path(__file__).resolve().parents[3]
class PumpkinFood(unittest.TestCase):
    def test_reachable_pumpkin_and_worm_have_exact_current_food(self):
        source = json.loads((ROOT / 'tools/content-schema/native-gameplay/pumpkin-skinning-item-profiles.json').read_bytes())
        for identifier, seconds in ((3594,204),(8177,12)):
            row = next(r for r in source['records'] if r['production_definition']['production_key'] == f'oteryn:item.tibia.i{identifier}')
            with self.subTest(identifier=identifier):
                self.assertEqual(row['semantics'].get('consumption'), {'state':'KNOWN','value':{'kind':'FOOD','value':{'regeneration_seconds':seconds}}})
if __name__ == '__main__':
    unittest.main()
