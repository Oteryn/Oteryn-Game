"""M14's census admission stays independent of the generated runtime union."""
import json
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import update_equipment_profile_additions as pins


class Srv7EquipmentCohortTests(unittest.TestCase):
    def test_committed_pin_contains_one_source_qualified_turnip(self):
        result = pins.expected()
        self.assertIn('("SRV7-M14", 1)', result)
        self.assertEqual(result, pins.TARGET.read_text())

    def test_missing_proof_never_lowers_the_expected_runtime_census(self):
        original = Path.read_bytes

        def read(path):
            raw = original(path)
            if path.name == 'srv7-targuna-item-profiles.json':
                packet = json.loads(raw)
                packet['records'] = []
                return json.dumps(packet).encode()
            return raw

        with patch.object(Path, 'read_bytes', read):
            with self.assertRaisesRegex(ValueError, 'one distinct zero-speed turnip'):
                pins.expected()


if __name__ == '__main__':
    unittest.main()
