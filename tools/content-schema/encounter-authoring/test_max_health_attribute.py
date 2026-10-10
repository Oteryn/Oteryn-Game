"""Accepted ENCOUNTER-RT-0 section 17.2 authoring admission boundary."""
import copy
import json
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator

from validate_encounter import validate


class MaxHealthAttributeTests(unittest.TestCase):
    def errors(self, attribute, operation, value=None, present=True):
        path = Path(__file__).parent / 'samples/goshnars_hatred/encounter.json'
        document = copy.deepcopy(json.loads(path.read_text(encoding='utf-8')))
        action = {'kind': 'attribute', 'role': 'goshnars_hatred',
                  'attribute': attribute, 'operation': operation}
        if present:
            action['value'] = value
        document['rules'][0]['actions'].append(action)
        return validate(document)

    def test_absolute_health_bounds(self):
        for value in (1, 60000, 4294967295):
            with self.subTest(value=value):
                self.assertEqual([], self.errors('max_health', 'set', value))

    def test_refuses_out_of_range_and_nonabsolute_values(self):
        for value in (0, -1, 4294967296, True, 60000.5, {'counter': 'hatred'}):
            with self.subTest(value=value):
                self.assertTrue(self.errors('max_health', 'set', value))
        self.assertTrue(self.errors('max_health', 'set', present=False))
        self.assertTrue(self.errors('max_health', 'add', 60000))

    def test_reset_takes_no_value(self):
        self.assertEqual([], self.errors('max_health', 'reset', present=False))
        self.assertTrue(self.errors('max_health', 'reset', 60000))

    def test_existing_attributes_keep_add_reset(self):
        for attribute in ('defense', 'outgoing_damage_percent'):
            with self.subTest(attribute=attribute):
                self.assertEqual([], self.errors(attribute, 'add', 10))
                self.assertEqual([], self.errors(attribute, 'add', {'counter': 'hatred'}))
                self.assertEqual([], self.errors(attribute, 'reset', present=False))
                self.assertTrue(self.errors(attribute, 'set', 10))
                self.assertTrue(self.errors(attribute, 'reset', 10))

    def test_monster_health_uses_the_same_native_bound(self):
        path = Path(__file__).parent.parent / 'monster-authoring/monster.schema.json'
        stats = json.loads(path.read_text(encoding='utf-8'))['$defs']['stats']['properties']
        for field in ('max_health', 'initial_health'):
            validator = Draft202012Validator(stats[field])
            for value in (1, 60000, 4294967295):
                self.assertEqual([], list(validator.iter_errors(value)))
            for value in (0, -1, 4294967296):
                self.assertTrue(list(validator.iter_errors(value)))


if __name__ == '__main__':
    unittest.main()
