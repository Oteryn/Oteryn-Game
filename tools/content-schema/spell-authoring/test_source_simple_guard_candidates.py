import copy
import json
from pathlib import Path
import unittest

import jsonschema
import source_simple_guard_candidates as producer

REPO = Path(__file__).resolve().parents[3]


class SourceGuardEvidenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.records = producer.build(REPO, '/workspace/spell-sources')
        cls.by_key = {row['registration_key']: row for row in cls.records}

    def row(self, donor, suffix):
        return next(row for key, row in self.by_key.items() if key.startswith(donor) and key.endswith(suffix + '#1'))

    def test_population_and_inactive_boundaries(self):
        self.assertEqual(len(self.records), 14)
        self.assertEqual(len(self.by_key), 14)
        for row in self.records:
            self.assertFalse(row['complete_spell_candidate'])
            self.assertFalse(row['runtime_activation'])
            self.assertFalse(row['native_execution_qualified'])
            self.assertEqual(row['historical_receipt_status'], 'BLOCKED')
            jsonschema.validate(row, producer.evidence_schema())

    def test_canary_rune_specific_refusals(self):
        intense = self.row('canary', 'runes/intense_healing_rune.lua')
        ultimate = self.row('canary', 'runes/ultimate_healing_rune.lua')
        self.assertEqual(intense['source_semantics']['monster_lookup_number_argument'], 1073762188)
        self.assertFalse(intense['source_semantics']['requires_player_conversion'])
        self.assertEqual(ultimate['source_semantics']['blocked_vocation_name'], 'exalted monk')
        expressions = [event['expression'] for event in ultimate['ordered_source_events']]
        self.assertTrue(any('Your vocation cannot use this rune.' in text for text in expressions))
        self.assertTrue(any('Monster(var:getNumber(1073762188))' in text for text in expressions))

    def test_remove_is_before_combat_not_dispel(self):
        for donor in ('canary', 'crystal'):
            row = self.row(donor, 'support/cancel_magic_shield.lua')
            events = row['ordered_source_events']
            remove = [event['start'] for event in events if event['identifier'] == 'creature:removeCondition']
            combat = [event['start'] for event in events if event['identifier'] == 'combat:execute']
            self.assertEqual(len(remove), 1); self.assertEqual(len(combat), 1)
            self.assertLess(remove[0], combat[0])
            self.assertTrue(row['source_semantics']['remove_before_combat'])

    def test_find_person_retains_source_275_and_conflict(self):
        for donor in ('canary', 'crystal'):
            row = self.row(donor, 'support/find_person.lua')
            self.assertEqual(row['source_semantics']['source_bands_tiles'], [5, 101, 275])
            self.assertEqual(row['source_semantics']['existing_descriptor_bands_tiles'], [5, 101, 251])
            self.assertTrue(any('275' in event['expression'] for event in row['source_binding_anchors']))

    def test_crystal_leiden_and_secondary_chronology_retained(self):
        nature = self.row('crystal', "healing/nature's_embrace.lua")
        self.assertTrue(nature['source_semantics']['secondary_helper_runs_even_when_primary_returns_false'])
        self.assertFalse(nature['source_semantics']['secondary_selector_has_viewport_check'])
        self.assertEqual(nature['source_semantics']['secondary_ratio'], '0.30')
        intense = self.row('crystal', 'runes/intense_healing_rune.lua')
        ultimate = self.row('crystal', 'runes/ultimate_healing_rune.lua')
        self.assertEqual(intense['source_semantics']['combat_variant_argument'], 'numeric_target_id')
        self.assertEqual(ultimate['source_semantics']['combat_variant_argument'], 'original_incoming_variant')
        self.assertEqual(ultimate['source_semantics']['leiden_direct_health_delta_formula']['magic_multiplier'], '12.4')

    def test_schema_rejects_promotion_wrong_values_and_unknown_fields(self):
        for field, value in [('complete_spell_candidate', True), ('runtime_activation', True), ('extra', 'field')]:
            row = copy.deepcopy(self.records[0]); row[field] = value
            with self.assertRaises(jsonschema.ValidationError): jsonschema.validate(row, producer.evidence_schema())
        row = copy.deepcopy(self.row('canary', 'support/find_person.lua'))
        row['source_semantics']['source_bands_tiles'][-1] = 251
        with self.assertRaises(jsonschema.ValidationError): jsonschema.validate(row, producer.evidence_schema())

    def test_source_identity_fails_closed(self):
        value = producer.SPECS[0]
        fact = {'registration_key': value['registration_key'], 'source_sha256': value['source_sha256'],
                'source_revision': producer.PINS[value['donor']]}
        with self.assertRaises(ValueError): producer.qualify(b'changed source', fact, value)


if __name__ == '__main__':
    unittest.main()
