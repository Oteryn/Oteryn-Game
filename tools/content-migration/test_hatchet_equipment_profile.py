import json
from pathlib import Path
import tempfile
import unittest

import hatchet_equipment_profile as hatchet

ROOT = Path(__file__).resolve().parents[2]


class HatchetEquipmentProfileTests(unittest.TestCase):
    def test_source_qualifies_unrestricted_hand_equipment(self):
        pattern, = hatchet.equipment(ROOT)['value']['patterns']['value']
        self.assertEqual(pattern['primary_slot'], {'state': 'KNOWN', 'value': 'WEAPON'})
        self.assertEqual(pattern['level'], {'state': 'KNOWN', 'value': 0})
        self.assertEqual(pattern['vocations'], {'state': 'KNOWN', 'value': []})

    def test_changed_source_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / hatchet.XML
            path.parent.mkdir(parents=True)
            path.write_text('<items/>')
            with self.assertRaisesRegex(ValueError, 'HATCHET_EQUIPMENT_SOURCE_PIN'):
                hatchet.equipment(root)

    def test_overlay_preserves_other_facts_and_is_idempotent(self):
        source = {'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': [{
            'production_definition': {'production_key': hatchet.KEY},
            'semantics': {'equipment': {'state': 'UNKNOWN'}, 'sentinel': 42},
        }]}
        generated = {'profile': json.dumps(source).encode(), 'manifest': b'{"item_profiles":{}}'}
        canonical = lambda value: json.dumps(value).encode()
        hatchet.admit(ROOT, generated, 'profile', 'manifest', canonical)
        first = generated.copy()
        hatchet.admit(ROOT, generated, 'profile', 'manifest', canonical)
        self.assertEqual(generated, first)
        self.assertEqual(json.loads(generated['profile'])['records'][0]['semantics']['sentinel'], 42)

    def test_overlay_refuses_conflicting_known_requirement(self):
        equipment = hatchet.equipment(ROOT)
        equipment['value']['patterns']['value'][0]['level']['value'] = 20
        document = {'records': [{'production_definition': {'production_key': hatchet.KEY},
                                'semantics': {'equipment': equipment}}]}
        generated = {'profile': json.dumps(document).encode(), 'manifest': b'{}'}
        with self.assertRaisesRegex(ValueError, 'NATIVE_PROFILE_FACT_CONFLICT'):
            hatchet.admit(ROOT, generated, 'profile', 'manifest', lambda value: json.dumps(value).encode())
