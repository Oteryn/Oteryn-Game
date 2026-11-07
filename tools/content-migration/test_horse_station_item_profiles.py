"""Keep31159 a real transparent source fixture, never a carried money/item grant."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest
HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('horse_profiles', HERE / 'horse_station_item_profiles.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

class HorseProfileTest(unittest.TestCase):
    def setUp(self):
        self.doc = json.loads((HERE.parent / 'content-schema/native-gameplay/horse-station-item-profiles.json').read_bytes())

    def test_actual_qualified_marker_and_blocking_fixture(self):
        module.validate(self.doc)
        self.assertEqual(len(self.doc['records']), 2)

    def test_marker_cannot_turn_into_carried_authority(self):
        altered = copy.deepcopy(self.doc)
        altered['records'][1]['admission']['legal_destinations'].append('Inventory')
        with self.assertRaisesRegex(ValueError, 'NO_INVENTORY'):
            module.validate(altered)

    def test_source_solid_cannot_be_turned_into_new_entry_geometry(self):
        altered = copy.deepcopy(self.doc)
        altered['records'][0]['attributes']['blocks_movement'] = False
        with self.assertRaisesRegex(ValueError, 'EXACT_OFFICIAL'):
            module.validate(altered)

    def test_unknown_or_duplicate_marker_source_is_refused(self):
        altered = copy.deepcopy(self.doc)
        altered['records'].append(copy.deepcopy(altered['records'][0]))
        with self.assertRaisesRegex(ValueError, 'COVERAGE'):
            module.validate(altered)

if __name__ == '__main__':
    unittest.main()
