import copy
import gzip
import json
from pathlib import Path
import unittest
import jsonschema
import source_monster_inline_semantics as source

ROOT = Path(__file__).resolve().parents[3]


class InlineSourceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        with gzip.open(ROOT / 'docs/reference/spells/r37-source-closure/unresolved-monster-spell-links.jsonl.gz', 'rt') as stream:
            cls.slots = [row for line in stream if (row := json.loads(line))['registered_source'] is None]
        cls.rows = [source.build_inline(slot) for slot in cls.slots]
        cls.schema = source.strict_schema(cls.rows)

    def test_exact_population_defaults_no_drain_substitution(self):
        self.assertEqual(19, len(self.rows))
        self.assertEqual(9, sum(row['source_type_resolution']['declared_type'] is not None for row in self.rows))
        for row in self.rows:
            self.assertEqual('COMBAT_UNDEFINEDDAMAGE', row['source_type_resolution']['effective_type'])
            self.assertFalse(row['source_type_resolution']['valid_drain_alias_substitution_used'])
            self.assertEqual('CONDITION_NONE', row['normalized_combat']['condition_type'])
            self.assertFalse(row['native_admission']); self.assertFalse(row['runtime_activation'])
            jsonschema.Draft202012Validator(self.schema).validate(row)

    def test_damage_bound_sort_preserves_source_signs(self):
        row = next(row for row in self.rows if row['source_parameters']['minDamage'] == 100)
        self.assertEqual(-565, row['normalized_combat']['min_combat_value'])
        self.assertEqual(100, row['normalized_combat']['max_combat_value'])
        self.assertEqual(100, row['source_parameters']['minDamage'])

    def test_area_order_and_direction_are_source_derived(self):
        row = next(row for row in self.rows if row['source_parameters'].get('length') == 8)
        self.assertTrue(row['normalized_combat']['need_direction'])
        self.assertEqual('length_spread', row['normalized_combat']['area_construction_order'][0]['shape'])
        row = next(row for row in self.rows if row['source_parameters'].get('radius') == 4)
        self.assertFalse(row['normalized_combat']['need_direction'])

    def test_valid_drain_or_condition_not_accepted_as_typo_cohort(self):
        for change in ({'type': '@COMBAT_LIFEDRAIN'}, {'condition': {'type': '@CONDITION_FIRE'}}, {'name': 'melee'}):
            slot = copy.deepcopy(self.slots[0]); slot['source_parameters'].update(change)
            with self.assertRaises(ValueError): source.build_inline(slot)

    def test_schema_rejects_runtime_and_element_guesses(self):
        for field, value in [('runtime_activation', True), ('extra', True)]:
            row = copy.deepcopy(self.rows[0]); row[field] = value
            with self.assertRaises(jsonschema.ValidationError): jsonschema.validate(row, self.schema)
        row = copy.deepcopy(self.rows[0]); row['source_type_resolution']['effective_type'] = 'COMBAT_LIFEDRAIN'
        with self.assertRaises(jsonschema.ValidationError): jsonschema.validate(row, self.schema)

    def test_exact_slot_rejects_fabricated_values_or_identity(self):
        slot = copy.deepcopy(self.slots[0]); slot['source_parameters']['minDamage'] = -999
        with self.assertRaisesRegex(ValueError, 'exact inline slot'): source.build_inline(slot)
        slot = copy.deepcopy(self.slots[0]); slot['slot_identity']['source_slot_index'] = 999
        with self.assertRaisesRegex(ValueError, 'exact inline slot'): source.build_inline(slot)


if __name__ == '__main__': unittest.main()
