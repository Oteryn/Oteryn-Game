"""Candidate executor requirements cannot be dropped from structurally valid source projections."""
import copy
import json
from pathlib import Path
import unittest

from validate_spell import validate


class CompletionSchemaTests(unittest.TestCase):
    def setUp(self):
        root = Path(__file__).parent/'samples/starter-bundles/instant-intense_healing'
        self.files = [json.loads((root/name).read_text()) for name in
                      ('spell.json', 'dependencies.json', 'catalog.json', 'manifest.json')]
        self.assertEqual(validate(*self.files), [])

    def errors(self, **presentation):
        files = copy.deepcopy(self.files)
        files[1]['effects'][0].setdefault('presentation', {}).update(presentation)
        return validate(*files)

    def test_caster_timing_and_binding_cannot_be_partially_lost(self):
        cue = 'canary.appearance:effect/magic_blue'
        self.assertEqual(self.errors(caster_effect_asset_binding=cue,
                                     caster_effect_timing='before_combat'), [])
        self.assertTrue(self.errors(caster_effect_asset_binding=cue))
        self.assertTrue(self.errors(caster_effect_timing='before_combat'))
        self.assertTrue(self.errors(caster_effect_asset_binding=cue,
                                    caster_effect_timing='after_combat'))

    def test_zero_health_obligation_cannot_be_disabled(self):
        for value in (False, None, 1):
            files = copy.deepcopy(self.files)
            files[1]['abilities'][0]['zero_damage_health_path'] = value
            self.assertTrue(validate(*files), value)
        files = copy.deepcopy(self.files)
        files[1]['abilities'][0]['zero_damage_health_path'] = True
        self.assertEqual(validate(*files), [])

    def test_selector_cannot_be_changed_to_a_different_target_model(self):
        files = copy.deepcopy(self.files)
        files[1]['abilities'][0]['target_selection'] = 'caster_or_top_creature'
        self.assertEqual(validate(*files), [])
        files[1]['abilities'][0]['target_selection'] = 'any_creature'
        self.assertTrue(validate(*files))


if __name__ == '__main__':
    unittest.main()
