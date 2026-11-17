import copy
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import source_gate_item_profiles as gate


class SourceGateItemProfiles(unittest.TestCase):
    def test_approved_addition_preserves_exact_historical_capture(self):
        current = gate.capture()
        binding_path = gate.ROOT / gate.BINDINGS
        historical = gate.historical_binding_bytes(binding_path.read_bytes(), gate.BINDINGS_SHA)
        read_bytes = Path.read_bytes

        def historical_read(path):
            return historical if path == binding_path else read_bytes(path)

        with patch.object(Path, 'read_bytes', historical_read):
            self.assertEqual(gate.capture(), current)
        self.assertIn({'path': gate.BINDINGS, 'sha256': gate.BINDINGS_SHA}, current['sources'])

    def test_unrelated_historical_binding_change_remains_refused(self):
        binding_path = gate.ROOT / gate.BINDINGS
        document = json.loads(binding_path.read_bytes())
        unrelated = next(row for row in document['bindings'] if row['external_id'] == '3031')
        self.assertNotIn(3031, gate.IDS)
        unrelated['source_revision'] = '0' * 40
        changed = (json.dumps(document, sort_keys=True, separators=(',', ':')) + '\n').encode()
        read_bytes = Path.read_bytes

        def changed_read(path):
            return changed if path == binding_path else read_bytes(path)

        with patch.object(Path, 'read_bytes', changed_read):
            with self.assertRaisesRegex(ValueError, 'Item binding table digest mismatch'):
                gate.capture()

    def test_source_pair_is_exact_ground_only_door_with_level_base(self):
        packet = gate.capture()
        self.assertEqual({r['production_definition']['production_key'] for r in packet['records']},
                         {f'oteryn:item.tibia.i{i}' for i in (1626,1627,1634,1635,1948,2572,2573,2574,2575,2576,2577,2578,2579,2580,5111,5112,21935)})
        for record in packet['records']:
            self.assertEqual(record['admission']['legal_destinations'], ['Ground'])
            self.assertEqual(record['semantics']['classification']['value']['item_type'],
                             ({'state': 'KNOWN', 'value': 'LADDER'} if record['production_definition']['production_key'].endswith('i1948') else {'state':'KNOWN','value':'DOOR'} if record['production_definition']['production_key'] in {'oteryn:item.tibia.i5111','oteryn:item.tibia.i5112'} else {'state':'NOT_APPLICABLE'}))
            self.assertEqual(record['production_binding']['disposition'], 'EXACT')

    def test_source_ladder_is_walkable_ground_only_and_exactly_bound(self):
        packet = gate.capture()
        ladder = next((r for r in packet['records'] if r['production_definition']['production_key'] == 'oteryn:item.tibia.i1948'), None)
        self.assertIsNotNone(ladder, 'native source ladder1948 must be admitted')
        self.assertEqual(ladder['semantics']['classification']['value']['item_type'],
                         {'state': 'KNOWN', 'value': 'LADDER'})
        self.assertFalse(ladder['attributes']['blocks_movement'])
        self.assertFalse(ladder['attributes']['blocks_projectile'])
        self.assertEqual(ladder['admission']['legal_destinations'], ['Ground'])
        self.assertEqual(ladder['semantics']['physical']['value']['pickupable'],
                         {'state': 'KNOWN', 'value': False})
        self.assertEqual(ladder['production_binding']['disposition'], 'EXACT')

    def test_source_courtyard_grass_and_archway_are_static_walkable(self):
        rows = {r['production_definition']['production_key']: r for r in gate.capture()['records']}
        for item in (1626,1627,1634,1635,21935):
            key=f'oteryn:item.tibia.i{item}'
            self.assertIn(key,set(rows),'source courtyard route needs its exact static profile')
            row=rows[key]
            self.assertFalse(row['attributes']['blocks_movement'])
            self.assertEqual(row['admission']['legal_destinations'],['Ground'])
            self.assertEqual(row['semantics']['physical']['value']['pickupable'], {'state':'KNOWN','value':False})
            self.assertEqual(row['semantics']['physical']['value']['movable'], {'state':'KNOWN','value':False})
            self.assertEqual(row['semantics']['classification']['value']['item_type'],{'state':'NOT_APPLICABLE'})
            self.assertEqual(row['production_binding']['disposition'],'EXACT')

    def test_source_red_carpet_family_is_static_walkable_with_exact_bindings(self):
        rows = {r['production_definition']['production_key']: r for r in gate.capture()['records']}
        for item in range(2572,2581):
            key = f'oteryn:item.tibia.i{item}'
            self.assertIn(key, set(rows), 'source floor-three carpet must have native movement facts')
            row = rows[key]
            self.assertFalse(row['attributes']['blocks_movement'])
            self.assertFalse(row['attributes']['blocks_projectile'])
            self.assertEqual(row['admission']['legal_destinations'], ['Ground'])
            self.assertEqual(row['semantics']['physical']['value']['movable'], {'state':'KNOWN','value':False})
            self.assertEqual(row['semantics']['physical']['value']['pickupable'], {'state':'KNOWN','value':False})
            self.assertEqual(row['production_binding']['disposition'], 'EXACT')

    def test_existing_profiles_preserved_and_conflicts_refuse(self):
        packet = gate.capture()
        canonical = lambda value: (json.dumps(value, sort_keys=True) + '\n').encode()
        original = {'records': [{'production_definition': {'family': 'Item', 'production_key': 'unrelated', 'revision_ref': 'definition-r1'},
                                  'semantics': {'accepted': True}}]}
        generated = {gate.PROFILE: canonical(original), gate.MANIFEST: canonical({'item_profiles': {}})}
        with patch.object(gate, 'capture', return_value=packet):
            gate.admit(gate.ROOT, generated, canonical)
            after = copy.deepcopy(generated)
            gate.admit(gate.ROOT, generated, canonical)
            self.assertEqual(generated, after)
            retained = next(r for r in json.loads(generated[gate.PROFILE])['records']
                            if r['production_definition']['production_key'] == 'unrelated')
            self.assertEqual(retained, original['records'][0])
            wrong = copy.deepcopy(packet)
            wrong['records'][0]['admission']['materializable'] = False
            with patch.object(gate, 'capture', return_value=wrong):
                with self.assertRaisesRegex(ValueError, 'EXISTING_CONFLICT'):
                    gate.admit(gate.ROOT, generated, canonical)


if __name__ == '__main__':
    unittest.main()
