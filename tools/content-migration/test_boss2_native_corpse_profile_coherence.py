"""Independent native providers must retain the canonical corpse MINT boundary."""
import json
import unittest

import elemental_spheres_profiles as spheres
import floor_assembly_profiles as floor
import qm50_elevator_profiles as elevator
import rafzane_elevator_profiles as rafzane


class NativeCorpseProfileCoherence(unittest.TestCase):
    def test_elevator_world_projection_providers_retain_the_same_admission(self):
        def records(path):
            return {row['production_definition']['production_key']: row
                    for row in json.loads(path.read_bytes())['records']}
        first = records(floor.ROOT / elevator.PATH)
        second = records(floor.ROOT / rafzane.PATH)
        self.assertEqual(set(first), {f'oteryn:item.tibia.i{i}' for i in elevator.IDS})
        for key, record in first.items():
            with self.subTest(item=key):
                self.assertEqual(record, second[key])
                self.assertIs(record['admission']['materializable'], False)

    def test_source_corpses_preserve_canonical_nonmaterializable_admission(self):
        packet = json.loads((floor.ROOT / floor.PATH).read_bytes())
        records = {row['production_definition']['production_key']: row
                   for row in packet['records']}
        canonical = {}
        for path in (floor.ROOT / 'content/items/definitions').glob('items-*.json'):
            for row in json.loads(path.read_bytes())['records']:
                definition = row['definition']
                canonical[definition['identity']['key']] = definition
        for item in sorted(floor.ELEMENTAL_CORPSES):
            key = f'oteryn:item.tibia.i{item}'
            with self.subTest(item=item):
                self.assertIs(canonical[key]['materializable'], False)
                self.assertEqual(records[key]['admission'], {
                    'materializable': False,
                    'stack_class': 'NonStackable',
                    'legal_destinations': ['Ground'],
                })

    def test_shared_corpse_profiles_agree_without_discarding_source_facts(self):
        def records(path):
            return {row['production_definition']['production_key']: row
                    for row in json.loads(path.read_bytes())['records']}
        assembly = records(floor.ROOT / floor.PATH)
        elemental = records(floor.ROOT / spheres.PATH)
        shared = floor.ELEMENTAL_CORPSES & spheres.IDS
        self.assertEqual(shared, {8106, 8139, 8140, 8141})
        for item in sorted(shared):
            key = f'oteryn:item.tibia.i{item}'
            with self.subTest(item=item):
                self.assertEqual(assembly[key], elemental[key])


if __name__ == '__main__':
    unittest.main()
