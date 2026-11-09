import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]

class RepairIntegrationProfiles(unittest.TestCase):
    def test_shared_elemental_decay_successors_preserve_stationary_admission(self):
        base = ROOT / 'tools/content-schema/native-gameplay'
        def records(name):
            return {r['production_definition']['production_key']: r for r in json.loads((base / name).read_text())['records']}
        floor = records('floor-assembly-item-profiles.json')
        elemental = records('elemental-spheres-item-profiles.json')
        for item in (8106, 8139, 8140, 8141):
            key = f'oteryn:item.tibia.i{item}'
            self.assertFalse(elemental[key]['admission']['materializable'])
            self.assertEqual(floor[key], elemental[key], key)

if __name__ == '__main__':
    unittest.main()
