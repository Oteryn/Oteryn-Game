import copy
import json
import unittest
from unittest.mock import patch

import source_gate_item_profiles as gate


class SourceGateItemProfiles(unittest.TestCase):
    def test_source_pair_is_exact_ground_only_door_with_level_base(self):
        packet = gate.capture()
        self.assertEqual({r['production_definition']['production_key'] for r in packet['records']},
                         {'oteryn:item.tibia.i5111', 'oteryn:item.tibia.i5112'})
        for record in packet['records']:
            self.assertEqual(record['admission']['legal_destinations'], ['Ground'])
            self.assertEqual(record['semantics']['classification']['value']['item_type'],
                             {'state': 'KNOWN', 'value': 'DOOR'})
            self.assertEqual(record['production_binding']['disposition'], 'EXACT')

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
