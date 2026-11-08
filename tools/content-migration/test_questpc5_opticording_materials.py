"""Opticording material custody uses exact identities and rejects source drift."""
import copy
import unittest
from pathlib import Path
from unittest.mock import patch
import questpc5_opticording_materials as materials

ROOT = Path(__file__).resolve().parents[2]

class OpticordingMaterials(unittest.TestCase):
    def test_spheres_spectacles_and_cube_are_five_exact_portable_instances(self):
        rows = materials.capture(ROOT)
        self.assertEqual([r['production_definition']['production_key'] for r in rows],
                         [f'oteryn:item.tibia.i{i}' for i in (19063,23572,23573,27522,33313)])
        self.assertEqual([r['semantics']['physical']['value']['weight']['value'] for r in rows], [200,200,180,250,1000])
        for row in rows:
            self.assertEqual(row['admission'], dict(materializable=True, stack_class='NonStackable', legal_destinations=['CharacterInventory','Ground']))
            identity = row['production_definition']
            self.assertEqual(row['production_binding']['target'], dict(family='Item',
                key=identity['production_key'], revision=identity['revision_ref']))
            number = identity['production_key'].rsplit('i', 1)[1]
            self.assertEqual(row['authoring']['item'], dict(family='Item',
                key=f'candidate:item/{number}', revision='spell-p2-r21'))
            self.assertEqual(row['production_binding']['disposition'], 'EXACT')
            self.assertEqual(row['semantics']['charges'], {'state':'NOT_APPLICABLE'})
            self.assertEqual(row['semantics']['temporal'], {'state':'NOT_APPLICABLE'})
            if number == '27522':
                self.assertEqual(row['semantics']['equipment']['value']['patterns']['value'][0]['primary_slot'],
                                 dict(state='KNOWN', value='HEAD'))

    def test_blue_spectacles_cannot_hide_a_different_script_registration(self):
        import xml.etree.ElementTree as xml
        definition = {'semantics': {'equipment': {'state': 'KNOWN', 'value': {'patterns': {'state': 'KNOWN', 'value': [
            {'primary_slot': {'state': 'KNOWN', 'value': 'HEAD'}}]}}}}}
        for slot in ('feet', 'backpack'):
            node = xml.fromstring(f'<item><attribute key="script" value="moveevent"><attribute key="slot" value="{slot}"/></attribute></item>')
            with self.subTest(slot=slot), self.assertRaisesRegex(ValueError, 'OPTICORDING_EXACT_HEAD_REGISTRATION'):
                materials.head.qualify(27522, node, {}, {}, {}, definition)

    def test_xml_pin_drift_is_not_a_named_item_match(self):
        original = Path.read_bytes
        def read(path):
            raw = original(path)
            return raw + b'\n' if path == ROOT / materials.shared.XML else raw
        with patch.object(Path, 'read_bytes', read):
            with self.assertRaisesRegex(ValueError, 'OPTICORDING_SOURCE_PIN'):
                materials.capture(ROOT)

    def test_served_appearance_cannot_be_a_container_or_immovable_alias(self):
        original = materials.shared.source.engine_items.load_appearance_objects
        for flag in ['flags.container','flags.unmove','flags.cumulative']:
            def load(raw):
                objects = original(raw)
                objects[19063] = copy.deepcopy(objects[19063])
                objects[19063]['flags'][flag] = True
                return objects
            with self.subTest(flag=flag), patch.object(materials.shared.source.engine_items, 'load_appearance_objects', load):
                with self.assertRaisesRegex(ValueError, 'OPTICORDING_CLIENT_CUSTODY'):
                    materials.capture(ROOT)

if __name__ == '__main__':
    unittest.main()
