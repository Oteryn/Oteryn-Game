"""Physical appearance evidence reader tests, independent of native Asset decisions."""
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import qualify_creature_assets as assets


def varint(number):
    output = bytearray()
    while number >= 128:
        output.append((number & 127) | 128)
        number >>= 7
    return bytes(output + bytes([number]))


def scalar(field, number):
    return varint(field << 3) + varint(number)


def message(field, data):
    return varint(field << 3 | 2) + varint(len(data)) + data


def appearance(family, number, sprites, packed=False):
    info = message(5, b''.join(varint(n) for n in sprites)) if packed else b''.join(scalar(5, n) for n in sprites)
    return message(family, scalar(1, number) + message(2, message(3, info)))


class AssetEvidenceTests(unittest.TestCase):
    def test_numeric_identity_uses_family_not_names(self):
        decoded = assets.decode_appearances(appearance(1, 35, [1, 2]) + appearance(2, 35, [3, 4]))
        self.assertEqual(decoded['object', 35]['sprite_ids'], [1, 2])
        self.assertEqual(decoded['outfit', 35]['sprite_ids'], [3, 4])

    def test_packed_and_unpacked_ids_have_same_values(self):
        plain = assets.decode_appearances(appearance(2, 35, [0, 130, 500]))
        packed = assets.decode_appearances(appearance(2, 35, [0, 130, 500], True))
        self.assertEqual(plain['outfit', 35]['sprite_ids'], packed['outfit', 35]['sprite_ids'])
        self.assertNotEqual(plain['outfit', 35]['record_sha256'], packed['outfit', 35]['record_sha256'])

    def test_duplicate_id_fails_closed(self):
        row = appearance(2, 35, [1])
        with self.assertRaisesRegex(ValueError, 'duplicated'):
            assets.decode_appearances(row + row)

    def test_missing_id_fails_closed(self):
        with self.assertRaisesRegex(ValueError, 'missing'):
            assets.decode_appearances(message(2, b''))

    def test_names_and_unsupported_tokens_do_not_resolve(self):
        self.assertIsNone(assets.binding_id('canary.appearance:outfit/demon', {}))
        self.assertIsNone(assets.binding_id('oteryn.appearance:outfit/35', {}))
        self.assertEqual(assets.binding_id('canary.appearance:outfit/35', {}), ('outfit', 35))

    def test_effects_require_exact_enum_constant(self):
        self.assertEqual(assets.binding_id('canary.appearance:effect/firearea', {'CONST_ME_FIREAREA': 7}), ('effect', 7))
        self.assertIsNone(assets.binding_id('canary.appearance:effect/firearea', {}))

    def test_declared_source_object_id_takes_priority_without_remapping(self):
        self.assertEqual(assets.declared_binding('Spyrat East', {'outfit': {'lookType': 1, 'lookTypeEx': 30375}}),
                         ('object', 30375))
        self.assertEqual(assets.declared_binding('Demon', {'outfit': {'lookType': 35}}), ('outfit', 35))

    def test_absent_look_resolves_only_accepted_familiar_default(self):
        self.assertIsNone(assets.declared_binding('Demon', {'outfit': {'lookType': 0}}))
        self.assertIsNone(assets.declared_binding('Unknown Familiar', {'flags': {'familiar': True}}))
        self.assertEqual(assets.declared_binding('Knight Familiar', {'flags': {'familiar': True}}), ('outfit', 991))

    def test_sprite_range_requires_one_exact_file(self):
        ranges = [{'file': 'a', 'firstspriteid': 0, 'lastspriteid': 10},
                  {'file': 'b', 'firstspriteid': 10, 'lastspriteid': 20}]
        files, missing = assets.sprite_files({'sprite_ids': [0, 10, 21]}, ranges)
        self.assertEqual(files, ['a'])
        self.assertEqual([row['sprite_id'] for row in missing], [10, 21])

    def test_protocol_evidence_is_exact_revision_and_required_semantics(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'protocol.cpp'
            source.write_bytes(b'header\ndirect object ID\n')
            with patch.object(assets.subprocess, 'check_output', return_value=source.read_bytes()):
                evidence = assets.pinned_code(root, 'revision', source.name, 'direct object ID')
                self.assertEqual(evidence['line'], 2)
                with self.assertRaisesRegex(ValueError, 'semantics'):
                    assets.pinned_code(root, 'revision', source.name, 'server-to-client remap')
            with patch.object(assets.subprocess, 'check_output', return_value=b'other revision'):
                with self.assertRaisesRegex(ValueError, 'exact revision'):
                    assets.pinned_code(root, 'revision', source.name, 'direct object ID')


if __name__ == '__main__':
    unittest.main()
