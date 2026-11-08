"""The admitted jade replica retains exact source speed instead of unknown equipment."""
import hashlib
import json
import unittest
import xml.etree.ElementTree as ET
from pathlib import Path

import quest_callback_profiles as profiles

ROOT = Path(__file__).resolve().parents[2]


class RaidReplicaSpeed(unittest.TestCase):
    def test_jade_source_and_generated_speed_are_qualified_together(self):
        raw = (ROOT / profiles.SOURCE).read_bytes()
        self.assertEqual(hashlib.sha256(raw).hexdigest(), profiles.SOURCE_SHA256)
        source = next(item for item in ET.fromstring(raw).findall('item')
                      if item.get('id') == '31268')
        self.assertEqual(source.get('name'), 'jade amulet')
        self.assertEqual(source.findall("attribute[@key='speed']"), [])
        rows = {row['production_definition']['production_key']: row for row in
                json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())['records']}
        jade = rows['oteryn:item.tibia.i31268']
        self.assertTrue(jade['admission']['materializable'])
        self.assertEqual(jade['attributes'].get('speed_bonus'), 0)
        for item in [9215, 9216, 9217, 9221]:
            record = rows[f'oteryn:item.tibia.i{item}']
            self.assertFalse(record['admission']['materializable'])
            self.assertNotIn('speed_bonus', record['attributes'])


if __name__ == '__main__':
    unittest.main()
