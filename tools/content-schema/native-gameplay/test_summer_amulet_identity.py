"""Closed identity-owner regressions; no donor network/runtime authority is mocked."""
import copy
import json
from pathlib import Path
import unittest
from build_spell_item_identities import BINDINGS, bind_profiles, binding_index
from summer_amulet_identity import PACKET, IDS, UNKNOWN_SECTIONS
ROOT = Path(__file__).resolve().parents[3]
class SummerAmuletQualification(unittest.TestCase):
    def setUp(self):
        self.packet = json.loads((ROOT / PACKET).read_bytes())
        self.bindings = binding_index(BINDINGS.read_bytes())
    def bind(self, rows=None, packet=None, bindings=None):
        return bind_profiles({'schema':'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records':self.packet['profiles'] if rows is None else rows},
            self.bindings if bindings is None else bindings, set(),
            summer_amulet_candidate=self.packet if packet is None else packet)
    def test_exact_five_custody_and_unknown_equipment(self):
        rows = self.bind()['records']
        self.assertEqual({int(row['production_binding']['external_id']) for row in rows}, IDS)
        for row in rows:
            self.assertEqual(row['admission'], {'materializable':True,'stack_class':'NonStackable','legal_destinations':['CharacterInventory','Ground']})
            for section in UNKNOWN_SECTIONS:
                self.assertEqual(row['semantics'][section], {'state':'UNKNOWN'})
    def test_existing_identity_without_qualification_still_refuses(self):
        with self.assertRaisesRegex(ValueError, 'Unbound authored Item reference'):
            bind_profiles({'schema':'OTERYN_NATIVE_ITEM_PROFILES/v1','records':self.packet['profiles']}, self.bindings, set())
    def test_companion_even_id_remains_unqualified(self):
        rows = copy.deepcopy(self.packet['profiles'])
        rows[0]['authoring']['item']['key'] = 'candidate:item/53198'
        with self.assertRaisesRegex(ValueError, 'Unbound'):
            self.bind(rows)
    def test_equipment_timer_or_weight_cannot_be_invented(self):
        for section in UNKNOWN_SECTIONS:
            rows = copy.deepcopy(self.packet['profiles'])
            rows[0]['semantics'][section] = {'state':'NOT_APPLICABLE'}
            with self.assertRaisesRegex(ValueError, 'physical-only'):
                self.bind(rows)
        rows = copy.deepcopy(self.packet['profiles'])
        rows[0]['semantics']['physical']['value']['weight']['value'] = 0
        with self.assertRaisesRegex(ValueError, 'physical-only'):
            self.bind(rows)
    def test_packet_expansion_and_historical_override_refuse(self):
        packet = copy.deepcopy(self.packet)
        packet['bindings'].append(copy.deepcopy(packet['bindings'][0]))
        with self.assertRaisesRegex(ValueError, 'Unqualified'):
            self.bind(packet=packet)
        bindings = copy.deepcopy(self.bindings)
        bindings['53197'] = self.packet['bindings'][0]
        with self.assertRaisesRegex(ValueError, 'conflicts'):
            self.bind(bindings=bindings)
if __name__ == '__main__':
    unittest.main()
