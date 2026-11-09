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

    def test_elevator_scenery_is_stationary_in_both_providers(self):
        base = ROOT / 'tools/content-schema/native-gameplay'
        def records(name):
            return {r['production_definition']['production_key']: r for r in json.loads((base / name).read_text())['records']}
        qm50 = records('qm50-elevator-item-profiles.json')
        rafzane = records('rafzane-elevator-item-profiles.json')
        self.assertEqual(len(qm50), 8)
        for key, row in qm50.items():
            self.assertFalse(row['admission']['materializable'], key)
            self.assertEqual(row, rafzane[key], key)

if __name__ == '__main__':
    unittest.main()
