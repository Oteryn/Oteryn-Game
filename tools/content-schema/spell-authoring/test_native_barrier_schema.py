"""Shared Effect validation fences the duration accepted by barrier planning."""
import copy
import json
from pathlib import Path
import unittest

from validate_spell import validate


class NativeBarrierSchemaTests(unittest.TestCase):
    def setUp(self):
        root = Path(__file__).parent / 'samples/starter-bundles/instant-intense_healing'
        self.files = [json.loads((root / name).read_text()) for name in
                      ('spell.json', 'dependencies.json', 'catalog.json')]
        deps = self.files[1]
        identity = deps['effects'][0]['identity']
        revision = identity['revision']
        normal = {'family': 'Item', 'key': 'candidate:item/2128', 'revision': revision}
        safe = {'family': 'Item', 'key': 'candidate:item/10181', 'revision': revision}
        deps['effects'] = [{
            'identity': identity, 'operation': 'create_item', 'created_item': normal,
            'pvp_safe_item': safe, 'duration_range_ms': {'minimum': 16000, 'maximum': 24000},
            'duration_selection': 'uniform_integer_seconds', 'safe_world_type': 'optional_pvp',
            'refuse_on': ['floor_change_tile', 'creature_on_tile'],
            'description_template': 'Casted by: {caster_name}',
            'presentation': {'projectile_asset_binding': 'canary.appearance:missile/energy'},
        }]
        deps['formulas'] = []
        deps['abilities'][0]['effects'] = [
            {'family': 'Effect', 'key': identity['key'], 'revision': revision}]
        self.files[2]['definitions'].extend([normal, safe])
        self.assertEqual(validate(*self.files), [])

    def errors(self, minimum, maximum):
        files = copy.deepcopy(self.files)
        files[1]['effects'][0]['duration_range_ms'] = {'minimum': minimum, 'maximum': maximum}
        return validate(*files)

    def test_largest_whole_second_u32_duration_is_admitted(self):
        self.assertEqual(self.errors(4294967000, 4294967000), [])

    def test_duration_exceeding_runtime_u32_is_rejected(self):
        self.assertTrue(self.errors(4294968000, 4294968000))

    def test_zero_duration_is_rejected(self):
        self.assertTrue(self.errors(0, 24000))

    def test_fractional_second_duration_is_rejected(self):
        self.assertTrue(self.errors(16001, 24000))

    def test_reversed_whole_second_range_is_rejected(self):
        errors = self.errors(24000, 16000)
        self.assertTrue(any('range is reversed' in error for error in errors), errors)


if __name__ == '__main__':
    unittest.main()
