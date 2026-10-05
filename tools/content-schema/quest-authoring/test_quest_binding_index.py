"""Reject stale, cross-quest and runtime-promoted authoring evidence."""
import copy
import unittest
from pathlib import Path
import jsonschema
from quest_binding_authoring import DIRECTORY, read, validate_index, dereference


class BindingIndexControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root = Path(__file__).resolve().parents[3]
        cls.scratch = cls.root / DIRECTORY
        cls.manifest = read(cls.scratch / 'index.json')

    def candidate(self):
        return copy.deepcopy(self.manifest)

    def test_negative_array_index_rejected(self):
        with self.assertRaisesRegex(ValueError, 'array index'):
            dereference({'records': [0, 1]}, '/records/-1')

    def test_runtime_promotion_rejected(self):
        value = self.candidate()
        value['records'][0]['runtime_admission'] = 'READY'
        with self.assertRaises(jsonschema.ValidationError):
            validate_index(self.root, self.scratch, value)

    def test_stale_definition_rejected(self):
        value = self.candidate()
        value['records'][0]['definition']['sha256'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'stale definition'):
            validate_index(self.root, self.scratch, value)

    def test_cross_quest_join_rejected(self):
        value = self.candidate()
        value['records'][0]['supplement_refs'][0]['json_pointer'] = '/records/1'
        with self.assertRaisesRegex(ValueError, 'another quest'):
            validate_index(self.root, self.scratch, value)

    def test_unregistered_output_rejected(self):
        value = self.candidate()
        value['outputs'][0]['path'] = 'outside/packet.json'
        with self.assertRaisesRegex(ValueError, 'membership'):
            validate_index(self.root, self.scratch, value)


if __name__ == '__main__':
    unittest.main()
