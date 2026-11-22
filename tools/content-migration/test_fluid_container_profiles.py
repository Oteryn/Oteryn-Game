"""I10a: the active provider exposes the carried fluid containers."""
import json
from pathlib import Path
import unittest
import fluid_container_profiles as fluids

ROOT = Path(__file__).resolve().parents[2]


class FluidContainerProfiles(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        rows = json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())['records']
        cls.profiles = {r['production_definition']['production_key']: r for r in rows}

    def test_unnatural_selection_pot_has_native_fluid_custody(self):
        row = self.profiles['oteryn:item.tibia.i3465']
        self.assertTrue(row['admission']['materializable'])
        self.assertEqual(row['admission']['stack_class'], 'NonStackable')
        self.assertEqual(row['semantics']['fluid'], fluids.known({'fluid_type': {'state': 'NOT_APPLICABLE'}}))

    def test_fluid_containers_are_carried_non_stackable_and_hold_an_instance_fluid(self):
        self.assertEqual(len(fluids.IDS), 21)
        self.assertNotIn(2524, fluids.IDS)
        for number in sorted(fluids.IDS):
            row = self.profiles[f'oteryn:item.tibia.i{number}']
            self.assertEqual(row['production_binding']['disposition'], 'EXACT')
            self.assertTrue(row['admission']['materializable'])
            self.assertEqual(row['admission']['stack_class'], 'NonStackable')
            self.assertEqual(row['admission']['legal_destinations'], ['CharacterInventory', 'Ground'])
            self.assertEqual(row['semantics']['fluid'], fluids.known({'fluid_type': {'state': 'NOT_APPLICABLE'}}))


if __name__ == '__main__':
    unittest.main()
