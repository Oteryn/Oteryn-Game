"""Regression checks for spell authoring parity and S24 source precedence."""
import copy
import unittest

from convert_spells import PARTY, Bundle, Wikis
from validate_spell import validate
from verify_formal_schema import CATALOG, POSITIVE, identity, ref


def party_bundle():
    bundle = copy.deepcopy(POSITIVE['light_healing'][0])
    effect_key = 'oteryn:effect.test.party'
    bundle['spell']['costs']['mana'] = 0
    bundle['spell']['execution'] = {'native_behavior': {'key': 'party_buff', 'parameters': {
        'area': PARTY['area'], 'same_floor': True, 'requires_party': True, 'min_affected': 2,
        'mana': {'mode': 'fixed', 'base': 75}, 'effect': ref('Effect', effect_key)}}}
    deps = {'abilities': [], 'formulas': [], 'effects': [{
        'identity': identity(effect_key), 'operation': 'condition', 'duration_ms': 120000,
        'condition': {'type': 'regeneration', 'lifetime': 'fixed_duration', 'buff_spell': True,
                      'regeneration': {'mana_gain': 2, 'mana_interval_ms': 2000}}}]}
    return bundle, deps


class SpellValidationTests(unittest.TestCase):
    def test_harmony_is_exclusive_to_monk_vocations(self):
        for role in ('builder', 'spender'):
            bundle, deps = copy.deepcopy(POSITIVE['light_healing'])
            bundle['spell']['harmony_role'] = role
            self.assertTrue(validate(bundle, deps, CATALOG))
            for vocations in (['monk'], ['exalted_monk'], ['exalted_monk', 'monk']):
                bundle['spell']['requirements']['vocations'] = vocations
                self.assertEqual(validate(bundle, deps, CATALOG), [])
            bundle['spell']['requirements']['vocations'] = ['knight', 'monk']
            self.assertTrue(validate(bundle, deps, CATALOG))

    def test_fixed_party_mana_has_one_charging_authority(self):
        bundle, deps = party_bundle()
        self.assertEqual(validate(bundle, deps), [])
        bundle['spell']['costs']['mana'] = 75
        self.assertTrue(validate(bundle, deps))

    def test_party_parameters_reject_missing_or_unknown_fields(self):
        bundle, deps = party_bundle()
        fields = tuple(bundle['spell']['execution']['native_behavior']['parameters'])
        for field in fields:
            with self.subTest(field=field):
                changed = copy.deepcopy(bundle)
                del changed['spell']['execution']['native_behavior']['parameters'][field]
                self.assertTrue(validate(changed, deps))
        bundle['spell']['execution']['native_behavior']['parameters']['unknown'] = True
        self.assertTrue(validate(bundle, deps))

    def test_party_parameter_types_and_boundaries(self):
        for field, value in (('same_floor', False), ('same_floor', 1), ('requires_party', False),
                             ('min_affected', 0), ('min_affected', True),
                             ('min_affected', 4294967296), ('area', []), ('area', ['c']),
                             ('area', ['Cx', 'x']), ('area', ['CC']), ('area', ['xxx'])):
            with self.subTest(field=field, value=value):
                bundle, deps = party_bundle()
                bundle['spell']['execution']['native_behavior']['parameters'][field] = value
                self.assertTrue(validate(bundle, deps))

    def test_party_effect_requires_its_exact_local_revision(self):
        bundle, deps = party_bundle()
        effect = bundle['spell']['execution']['native_behavior']['parameters']['effect']
        effect['revision'] = 'definition-r2'
        self.assertTrue(validate(bundle, deps))
        deps['effects'] = []
        self.assertIn('party_buff effect needs a local Effect payload',
                      validate(bundle, deps, {'definitions': [effect]}))

    def test_scaled_mana_accepts_whole_percent_only(self):
        for falloff in (0.01, 0.29, 0.9, 1):
            bundle, deps = party_bundle()
            bundle['spell']['execution']['native_behavior']['parameters']['mana'] = {
                'mode': 'scaled', 'base': 120, 'falloff': falloff, 'rounding': 'up'}
            self.assertEqual(validate(bundle, deps), [], falloff)
        for mana in ({'mode': 'fixed', 'base': True}, {'mode': 'fixed', 'base': 75, 'falloff': 0.9},
                     {'mode': 'scaled', 'base': 120, 'falloff': 0, 'rounding': 'up'},
                     {'mode': 'scaled', 'base': 120, 'falloff': 0.905, 'rounding': 'up'},
                     {'mode': 'scaled', 'base': 120, 'falloff': 0.9, 'rounding': 'down'}):
            with self.subTest(mana=mana):
                bundle, deps = party_bundle()
                bundle['spell']['execution']['native_behavior']['parameters']['mana'] = mana
                self.assertTrue(validate(bundle, deps))

    def test_ability_matrices_need_one_centre_and_equal_rows(self):
        for matrix in (['Cx', 'x'], ['CC'], ['xxx']):
            bundle, deps = copy.deepcopy(POSITIVE['light_healing'])
            deps['abilities'][0]['area'] = {'matrix': {'north': matrix}}
            self.assertTrue(validate(bundle, deps, CATALOG), matrix)
        deps['abilities'][0]['area'] = {'matrix': {'north': ['xCx']}}
        self.assertEqual(validate(bundle, deps, CATALOG), [])

    def test_windup_rejects_area_and_missing_target(self):
        bundle, deps = copy.deepcopy(POSITIVE['light_healing'])
        deps['abilities'][0]['windup'] = {'delay_ms': 1, 'caster_asset_binding': 'test:effect.cast'}
        self.assertTrue(any('/windup: only on' in e for e in validate(bundle, deps, CATALOG)))
        deps['abilities'][0]['needs_target'] = True
        self.assertEqual(validate(bundle, deps, CATALOG), [])
        deps['abilities'][0]['area'] = {'radius_tiles': 1}
        self.assertTrue(any('/windup: only on' in e for e in validate(bundle, deps, CATALOG)))

    def test_variants_require_local_payloads_and_cannot_nest(self):
        bundle, deps = copy.deepcopy(POSITIVE['light_healing'])
        ability = deps['abilities'][0]
        variant_template = copy.deepcopy(ability)
        del ability['effects']
        ability['variants'] = [ref('Ability', 'oteryn:ability.test.one'),
                               ref('Ability', 'oteryn:ability.test.two')]
        errors = validate(bundle, deps, CATALOG)
        self.assertTrue(any('variant needs a local Ability payload' in e for e in errors))
        for reference in ability['variants']:
            variant = copy.deepcopy(variant_template)
            variant['identity'] = {k: reference[k] for k in ('key', 'revision')}
            deps['abilities'].append(variant)
        self.assertEqual(validate(bundle, deps, CATALOG), [])
        del deps['abilities'][1]['effects']
        deps['abilities'][1]['variants'] = copy.deepcopy(ability['variants'])
        self.assertTrue(any('a variant cannot have variants' in e for e in validate(bundle, deps, CATALOG)))


class CalculatorDecisionTests(unittest.TestCase):
    def candidate(self, name='strong ethereal spear'):
        return Bundle(name, {'canary': {}}, None, {}, {})

    def test_owner_decision_sets_power_and_attributes_disagreement(self):
        for previous in (38, 25):
            bundle = self.candidate()
            self.assertEqual(bundle.selected_base_power('instant', previous), 25)
            self.assertIn('BP38 remains unchanged', bundle.rows[-1]['resolution'])
            self.assertEqual(bundle.rows[-1]['destination'], '/spell/spell/base_power')
            self.assertEqual(bundle.sources[-1]['revision'],
                             'eeed345c86a76eb00f5ff7b2f0e0b49927799bf1')

    def test_decision_does_not_change_other_spells_or_carriers(self):
        for name, carrier in (('ethereal spear', 'instant'), ('strong ethereal spear', 'rune')):
            bundle = self.candidate(name)
            self.assertEqual(bundle.selected_base_power(carrier, 38), 38)
            self.assertEqual(bundle.rows, [])

    def test_unqualified_power_does_not_inherit_the_decision(self):
        for previous in (None, True, '38', 0, 26, float('inf')):
            bundle = self.candidate()
            self.assertEqual(bundle.selected_base_power('instant', previous), previous)
            self.assertEqual(bundle.rows, [])


class SourcePrecedenceTests(unittest.TestCase):
    def resolver(self, changes=None):
        official = {'changes': changes or [{'spell': 'test spell', 'field': 'mana', 'value': '60',
                     'date': '2026-07-07', 'source': 'https://www.tibia.com/news/', 'fact': 'mana changed'}]}
        return Wikis({'pages': [], 'target_cut': '2026-09-27'}, {'pages': []}, official)

    def pages(self, fandom=None, br=None, library=None):
        def page(value):
            return {'fields': {'mana': str(value)}, 'timestamp': '2026-09-01'} if value is not None else None
        return {'fandom': page(fandom), 'br': page(br), 'tibiacom': page(library)}

    def test_official_change_overrides_agreement_disagreement_and_missing_values(self):
        for values in ((40, 40), (40, 50), (None, 40), (None, None)):
            with self.subTest(values=values):
                value, _, note = self.resolver().resolve(self.pages(*values), 'mana', 'test spell')
                self.assertEqual(value, 60)
                self.assertIn('S24:', note)

    def test_later_official_library_still_supersedes_announcement(self):
        value, provenance, _ = self.resolver().resolve(self.pages(40, 40, 75), 'mana', 'test spell')
        self.assertEqual(value, 75)
        self.assertEqual(provenance[0][0], 'tibiacom')

    def test_future_announcement_does_not_change_target_date(self):
        resolver = self.resolver()
        resolver.official[('test spell', 'mana')]['date'] = '2026-09-29'
        self.assertEqual(resolver.resolve(self.pages(40, 40), 'mana', 'test spell')[0], 40)

    def test_override_without_wiki_provenance_records_resolution(self):
        class Candidate(Bundle):
            def source_value(self, method, transform=lambda v: v):
                return {'canary': 40}
            def row(self, *args, **kwargs):
                self.rows.append((args, kwargs))
            def branch_vote(self, method):
                return None
        bundle = Candidate('test spell', {'canary': {}}, self.resolver(), {}, {})
        self.assertEqual(bundle.field('/spell/spell/costs/mana', 'mana', 'mana', self.pages()), 60)
        self.assertTrue(any('S24:' in str(row) for row in bundle.rows))


if __name__ == '__main__':
    unittest.main()
