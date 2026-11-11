import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]

class RepairIntegrationProfiles(unittest.TestCase):
    def test_shared_elemental_decay_successors_preserve_nonportable_admission(self):
        base = ROOT / 'tools/content-schema/native-gameplay'
        def records(name):
            return {r['production_definition']['production_key']: r for r in json.loads((base / name).read_text())['records']}
        floor = records('floor-assembly-item-profiles.json')
        elemental = records('elemental-spheres-item-profiles.json')
        for item in (8106, 8139, 8140, 8141):
            key = f'oteryn:item.tibia.i{item}'
            self.assertEqual(elemental[key]['admission'], {
                'materializable': True, 'stack_class': 'NonStackable',
                'legal_destinations': ['Ground']})
            self.assertTrue(elemental[key]['semantics']['physical']['value']['movable']['value'])
            self.assertFalse(elemental[key]['semantics']['physical']['value']['pickupable']['value'])
            self.assertEqual(floor[key], elemental[key], key)
        corpses = {f'oteryn:item.tibia.i{i}' for i in (8106, 8139, 8140, 8141)}
        for key in (floor.keys() & elemental.keys()) - corpses:
            self.assertEqual(elemental[key]['admission'], {
                'materializable': False, 'stack_class': 'NonStackable',
                'legal_destinations': ['Ground']})
            self.assertEqual(floor[key], elemental[key], key)

    def test_elevator_scenery_is_stationary_in_both_providers(self):
        base = ROOT / 'tools/content-schema/native-gameplay'
        def records(name):
            return {r['production_definition']['production_key']: r for r in json.loads((base / name).read_text())['records']}
        qm50 = records('qm50-elevator-item-profiles.json')
        rafzane = records('rafzane-elevator-item-profiles.json')
        self.assertEqual(set(qm50), {f'oteryn:item.tibia.i{i}' for i in
                         (6218, 17938, 17939, 17940, 17942, 17943, 17944, 17947, 17970)})
        for key, row in qm50.items():
            self.assertFalse(row['admission']['materializable'], key)
            self.assertEqual(row, rafzane[key], key)

    def test_crafting_overlap_preserves_accepted_canonical_presentation(self):
        base = ROOT / 'tools/content-schema/native-gameplay'
        def records(name):
            return {r['production_definition']['production_key']: r for r in json.loads((base / name).read_text())['records']}
        reward = records('qengine-reward-item-profiles.json')
        crafting = records('qready1-craft-item-profiles.json')
        for key in ('oteryn:item.tibia.i21506', 'oteryn:item.tibia.i21507'):
            self.assertEqual(reward[key], crafting[key], key)

if __name__ == '__main__':
    unittest.main()
