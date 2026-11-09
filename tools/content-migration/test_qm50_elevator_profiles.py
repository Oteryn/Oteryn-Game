import json
import unittest

import qm50_elevator_profiles as elevator


class ElevatorProfilesTest(unittest.TestCase):
    def test_qm50_source_profiles_keep_winch_and_empty_frame_solid(self):
        packet = elevator.capture()
        self.assertEqual(packet, json.loads((elevator.ROOT / elevator.PATH).read_bytes()))
        records = {int(row['production_definition']['production_key'].split('.i')[-1]): row
                   for row in packet['records']}
        self.assertEqual(set(records), elevator.IDS)
        for identifier, row in records.items():
            self.assertEqual(row['attributes']['blocks_movement'], identifier in {17940, 17942, 17944})
            self.assertFalse(row['semantics']['physical']['value']['pickupable']['value'])
            self.assertFalse(row['semantics']['physical']['value']['movable']['value'])
            self.assertEqual(row['admission']['legal_destinations'], ['Ground'])


if __name__ == '__main__':
    unittest.main()
