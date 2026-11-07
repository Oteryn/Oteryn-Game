"""Admission preserves S17 condition payloads and optional authored Bestiary notes."""
import json
import unittest

import creature_admission_stage as stage


class ConditionPreservation(unittest.TestCase):
    def test_light_and_false_buff_flag_survive_inline_staging(self):
        source = {'type': 'light', 'lifetime': 'fixed_duration',
                  'light': {'level': 9, 'color': 215}, 'buff_spell': False}
        result = stage.inline_effect({'identity': {'key': 'canary:effect/demon/defense-1'},
                                     'operation': 'condition', 'duration_ms': 120000,
                                     'condition': source}, stage.Mapper({}))
        self.assertEqual({'condition_type': 'light', 'lifetime': 'FixedDuration',
                          'light': source['light'], 'buff_spell': False}, result['operation']['condition'])
        self.assertEqual(120000, result['operation']['duration_ms'])

    def test_health_and_mana_regeneration_are_preserved(self):
        for payload in ({'health_gain': 20, 'health_interval_ms': 2000},
                        {'mana_gain': 5, 'mana_interval_ms': 3000},
                        {'health_gain': 20, 'health_interval_ms': 2000,
                         'mana_gain': 5, 'mana_interval_ms': 3000}):
            with self.subTest(payload=payload):
                result = stage.condition({'type': 'regeneration', 'lifetime': 'fixed_duration',
                                          'regeneration': payload, 'buff_spell': True}, stage.Mapper({}))
                self.assertEqual(payload, result['regeneration'])
                self.assertIs(True, result['buff_spell'])

    def test_attribute_buff_preserves_modifier_and_buff_flag(self):
        result = stage.condition({'type': 'attributes', 'lifetime': 'fixed_duration', 'buff_spell': True,
                                  'attribute_modifiers': [{'attribute': 'shield', 'mode': 'add', 'value': 3}]},
                                 stage.Mapper({}))
        self.assertEqual([{'attribute': 'shield', 'mode': 'Add', 'value': 3}], result['attribute_modifiers'])
        self.assertIs(True, result['buff_spell'])

    def test_legacy_condition_has_no_invented_optional_payloads(self):
        self.assertEqual({'condition_type': 'invisible', 'lifetime': 'FixedDuration'},
                         stage.condition({'type': 'invisible', 'lifetime': 'fixed_duration'}, stage.Mapper({})))

    def test_authored_notes_are_preserved_separately_from_locations(self):
        sample = stage.ROOT / 'tools/content-schema/monster-authoring/samples/canary-47dfd51f/dragon/monster.json'
        monster = json.loads(sample.read_text(encoding='utf-8'))
        bestiary = monster['creature']['bestiary']
        bestiary['notes'] = 'Oteryn-authored narrative.\nSecond paragraph.'
        bestiary['locations'] = 'Source location text.'
        corpse = int(monster['creature']['corpse_item']['key'].rsplit('/', 1)[1])
        residue = int(monster['creature']['death_residue']['item']['key'].rsplit('/', 1)[1])
        mapper = stage.Mapper({corpse: 'oteryn:item.dragon_corpse', residue: 'oteryn:item.blood'})
        result = stage.Stage(mapper).creature_profile(monster)['details']['bestiary']
        self.assertEqual(bestiary['notes'], result['notes'])
        self.assertEqual(bestiary['locations'], result['locations'])
        del bestiary['notes']
        self.assertNotIn('notes', stage.Stage(mapper).creature_profile(monster)['details']['bestiary'])


if __name__ == '__main__':
    unittest.main()
