"""Finite current-Santa state facts, qualified against the real pinned capture."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
import srv7_seasonal_toy_loot as subject


class SeasonalToyLoot(unittest.TestCase):
    def setUp(self):
        source = json.loads((subject.ROOT / subject.SOURCE).read_bytes())
        self.rows = [copy.deepcopy(row) for row in source['records']
                     if row['production_definition']['production_key'] in {
                         'oteryn:item.tibia.i6511', 'oteryn:item.tibia.i6568'}]
        self.assertEqual(len(self.rows), 2)
        self.facts = {}
        for row in self.rows:
            ref = row['production_definition']
            self.facts[ref['production_key']] = {
                'item': {'family': ref['family'], 'key': ref['production_key'], 'revision': ref['revision_ref']},
                'materializable': False, 'container_capacity': None}
        self.facts['oteryn:item.tibia.i5791'] = {'item': {}, 'materializable': False}

    def apply(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)
            (path / 'spell-native-profiles.json').write_text(json.dumps({
                'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': self.rows}))
            subject.admit(self.facts, path)

    def test_exact_source_canonical_inventory_states_are_materializable(self):
        self.apply()
        self.assertTrue(self.facts['oteryn:item.tibia.i6511']['materializable'])
        self.assertTrue(self.facts['oteryn:item.tibia.i6568']['materializable'])
        self.assertFalse(self.facts['oteryn:item.tibia.i5791']['materializable'])

    def test_changed_canonical_binding_is_rejected(self):
        self.rows[0]['production_binding']['disposition'] = 'ADAPTED'
        with self.assertRaises(ValueError):
            self.apply()

    def test_changed_pickup_physics_cannot_create_loot_admission(self):
        self.rows[0]['semantics']['physical']['value']['pickupable']['value'] = False
        with self.assertRaises(ValueError):
            self.apply()

    def test_changed_legal_destination_is_rejected(self):
        self.rows[0]['admission']['legal_destinations'] = ['Ground']
        with self.assertRaises(ValueError):
            self.apply()


if __name__ == '__main__':
    unittest.main()
