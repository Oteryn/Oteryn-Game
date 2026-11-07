"""Regression for source ItemID_t constants in the shared Combat converter."""
import copy
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest

import convert_spells  # installs the shared monster-authoring module path
import canary_batch


ROOT = Path(__file__).resolve().parent
FIELD_IDS = {'fire': 2118, 'energy': 2122, 'poison': 105}
UTILS = '''
enum MagicEffectClasses {
    CONST_ME_NONE = 0,
    CONST_ME_HITBYFIRE = 16,
    CONST_ME_ENERGYHIT = 12,
    CONST_ME_GREEN_RINGS = 9,
};
enum ShootType_t {
    CONST_ANI_NONE = 0,
    CONST_ANI_FIRE = 4,
    CONST_ANI_ENERGYBALL = 36,
    CONST_ANI_POISON = 15,
};
enum ItemID_t : uint16_t {
    ITEM_FIREFIELD_PVP_FULL = 2118,
    ITEM_ENERGYFIELD_PVP = 2122,
    ITEM_POISONFIELD_PVP = 105,
    ITEM_OTHER = 50000,
};
'''
ENUMS = {
    'CombatParam_t': {'COMBAT_PARAM_TYPE': 0, 'COMBAT_PARAM_EFFECT': 1,
                      'COMBAT_PARAM_DISTANCEEFFECT': 2, 'COMBAT_PARAM_CREATEITEM': 3},
    'CombatType_t': {'COMBAT_NONE': 0, 'COMBAT_FIREDAMAGE': 1,
                     'COMBAT_ENERGYDAMAGE': 2, 'COMBAT_EARTHDAMAGE': 4},
    'ConditionType_t': {'CONDITION_NONE': 0},
}


class FieldItemConversionTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        root = Path(self.directory.name)
        path = root / canary_batch.EFFECT_CONSTANTS
        path.parent.mkdir(parents=True)
        path.write_text(UTILS)
        self.converter = canary_batch.Converter(root, {}, {}, {}, {})
        self.converter.spell_scripts = SimpleNamespace(enums=copy.deepcopy(ENUMS))
        self.converter.pending_definitions = set()

    def convert(self, parameters):
        combat = {'param_calls': list(parameters.items()), 'callbacks': {}, 'conditions': []}
        deps = {'abilities': [], 'effects': [], 'formulas': []}
        notes = []
        self.converter.combat_ability('candidate:ability/test', combat,
                                      {'needs_target': False, 'needs_direction': False},
                                      0, deps, lambda asset: asset, notes)
        return deps, notes

    def test_all_nine_field_runes_from_both_captured_source_censuses(self):
        census = json.loads((ROOT / 'samples/spell-census-canary-99902524-crystal-ff7ede5.json').read_text())
        for source in ('canary', 'crystal'):
            records = [r for r in census[source]
                       if r.get('file', '').startswith('data/scripts/runes/')
                       and any(r['file'].endswith('/' + element + '_' + kind + '.lua')
                               for element in FIELD_IDS for kind in ('field', 'bomb', 'wall'))]
            self.assertEqual(len(records), 9)
            for record in records:
                with self.subTest(source=source, file=record['file']):
                    element = Path(record['file']).stem.split('_')[0]
                    deps, notes = self.convert(record['combats'][0]['parameters'])
                    created = [e['created_item'] for e in deps['effects'] if e['operation'] == 'create_item']
                    self.assertEqual([r['key'] for r in created], [f'canary:item/{FIELD_IDS[element]}'])
                    self.assertIn(('Item', created[0]['key']), self.converter.pending_definitions)
                    self.assertTrue(any('ItemID_t binds' in n for n in notes))
                    self.assertFalse(any('not a Canary constant' in n for n in notes))

    def test_item_ids_follow_the_selected_header_and_support_nonfield_items(self):
        header = self.converter.canary / canary_batch.EFFECT_CONSTANTS
        header.write_text(UTILS.replace('ITEM_FIREFIELD_PVP_FULL = 2118', 'ITEM_FIREFIELD_PVP_FULL = 40001'))
        changed = canary_batch.Converter(self.converter.canary, {}, {}, {}, {})
        changed.spell_scripts = self.converter.spell_scripts
        changed.pending_definitions = set()
        self.converter = changed
        for value, expected in [('ITEM_FIREFIELD_PVP_FULL', 40001), ('ITEM_OTHER', 50000), (2122, 2122)]:
            with self.subTest(value=value):
                deps, _ = self.convert({'COMBAT_PARAM_CREATEITEM': value})
                self.assertEqual(deps['effects'][0]['created_item']['key'], f'canary:item/{expected}')

    def test_missing_or_zero_item_values_fail_before_emitting_definitions(self):
        for value in ('ITEM_UNDEFINED', None, 0, -1):
            with self.subTest(value=value):
                self.converter.pending_definitions.clear()
                with self.assertRaisesRegex(canary_batch.SpellUnresolved, 'positive item id'):
                    self.convert({'COMBAT_PARAM_TYPE': 'COMBAT_FIREDAMAGE', 'COMBAT_PARAM_CREATEITEM': value})
                self.assertEqual(self.converter.pending_definitions, set())

    def test_unknown_non_item_parameters_keep_the_engine_nil_observation(self):
        notes = []
        params = self.converter.engine_params([['COMBAT_PARAM_EFFECT', 'CONST_ME_UNDEFINED']], notes)
        self.assertEqual(params, {'COMBAT_PARAM_EFFECT': 0})
        self.assertTrue(any('nil' in n for n in notes))


if __name__ == '__main__':
    unittest.main()
