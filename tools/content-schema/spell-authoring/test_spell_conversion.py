"""Regression checks for spell authoring parity and S24 source precedence."""
import copy
import json
import tempfile
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator

from convert_spells import MONSTER, PARTY, RUNE_USE_RESOLUTIONS, CONJURE_RESOLUTIONS, Bundle, Execution, Unresolved, Wikis
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


class ConjureItemTypeTests(unittest.TestCase):
    def candidate(self, root, name='lightest magic missile'):
        execution = Execution.__new__(Execution)
        execution.root = root
        execution.canonical_visuals = lambda text: text
        records = {'canary': {'spell_type': 'instant', 'file': 'conjure.lua', 'registrar': {},
                   'cast': {'tier': 'conjure', 'conjure': {'result_item_id': 3174, 'count': 10,
                            'effect': 'CONST_ME_MAGIC_BLUE', 'reagent_item_id': 3147}}}}
        wikis = Wikis({'pages': [], 'target_cut': '2026-09-27'}, {'pages': []}, {'changes': []})
        return Bundle(name, records, wikis, {'canary': execution}, {('canary', 'conjure.lua'): ''})

    def item_source(self, root, xml):
        target = Path(root) / 'data/items/items.xml'
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(xml, encoding='utf-8')

    def test_unregistered_xml_rune_overrides_blue_with_source_evidence(self):
        with tempfile.TemporaryDirectory() as root:
            self.item_source(root, '<items><item id="3174"><attribute key="type" value="rune"/></item></items>')
            bundle = self.candidate(root)
            old_records = copy.deepcopy(bundle.records)
            result = bundle.execution({}, None, {})['conjure']
            self.assertEqual(result['effect_asset_binding'], 'canary.appearance:effect/magic_red')
            self.assertEqual(result['count'], 10)
            self.assertEqual(bundle.records, old_records)
            row = next(r for r in bundle.rows if r['source_field'] == 'type')
            self.assertEqual(row['source_file'], 'data/items/items.xml')
            self.assertIn('Git blob', row['resolution'])
            self.assertIn('SHA256', row['resolution'])

    def test_non_rune_preserves_explicit_cue(self):
        with tempfile.TemporaryDirectory() as root:
            self.item_source(root, '<items><item id="3174"><attribute key="type" value="container"/></item></items>')
            result = self.candidate(root).execution({}, None, {})['conjure']
            self.assertEqual(result['effect_asset_binding'], 'canary.appearance:effect/magic_blue')

    def test_omitted_optional_argument_emits_red_for_xml_blank_rune(self):
        with tempfile.TemporaryDirectory() as root:
            self.item_source(root, '<items><item id="3147"><attribute key="type" value="rune"/></item></items>')
            bundle = self.candidate(root, 'blank rune')
            conjure = bundle.records['canary']['cast']['conjure']
            del conjure['effect']
            conjure.update(result_item_id=3147, reagent_item_id=0, count=1)
            old_records = copy.deepcopy(bundle.records)
            body = bundle.execution({}, None, {})['conjure']
            self.assertEqual(body['effect_asset_binding'], 'canary.appearance:effect/magic_red')
            self.assertEqual(body['result']['key'], 'candidate:item/3147')
            self.assertEqual(body['count'], 1)
            self.assertEqual(bundle.records, old_records)
            self.assertTrue(any(r['source_file'] == 'data/items/items.xml' for r in bundle.rows))

    def test_item_ranges_are_source_local_and_do_not_supply_unknown_ids(self):
        with tempfile.TemporaryDirectory() as root:
            self.item_source(root, '<items><item fromid="3174" toid="3175"><attribute key="type" value="rune"/></item></items>')
            engine = self.candidate(root).executions['canary']
            self.assertEqual(engine.conjure_effect(3175, None), ('CONST_ME_MAGIC_RED', True))
            self.assertEqual(engine.conjure_effect(3176, 'CONST_ME_MAGIC_BLUE'), ('CONST_ME_MAGIC_BLUE', None))
        with tempfile.TemporaryDirectory() as other_root:
            self.item_source(other_root, '<items><item id="3174"/></items>')
            engine = self.candidate(other_root).executions['canary']
            self.assertEqual(engine.conjure_effect(3174, None), (None, False))

    def test_missing_item_is_explicitly_unresolved_never_fabricated_red(self):
        with tempfile.TemporaryDirectory() as root:
            self.item_source(root, '<items><item id="3175"><attribute key="type" value="rune"/></item></items>')
            bundle = self.candidate(root)
            result = bundle.execution({}, None, {})['conjure']
            self.assertEqual(result['effect_asset_binding'], 'canary.appearance:effect/magic_blue')
            self.assertTrue(any(r['status'] == 'unresolved_dependency' for r in bundle.rows))

    def test_first_loaded_duplicate_or_overlapping_range_controls_success_cue(self):
        rune = '<attribute key="type" value="rune"/>'
        other = '<attribute key="type" value="container"/>'
        cases = [
            (f'<item id="3174">{other}</item><item id="3174">{rune}</item>', False, None),
            (f'<item id="3174">{rune}</item><item id="3174">{other}</item>', True, None),
            (f'<item fromid="3174" toid="3175">{rune}</item><item id="3174">{other}</item>', True, True),
            (f'<item id="3174">{other}</item><item fromid="3174" toid="3175">{rune}</item>', False, True),
        ]
        for declarations, first_is_rune, next_is_rune in cases:
            with self.subTest(xml=declarations), tempfile.TemporaryDirectory() as root:
                self.item_source(root, '<items>' + declarations + '</items>')
                bundle = self.candidate(root)
                body = bundle.execution({}, None, {})['conjure']
                self.assertEqual(body['effect_asset_binding'], 'canary.appearance:effect/' +
                                 ('magic_red' if first_is_rune else 'magic_blue'))
                self.assertEqual(bundle.executions['canary'].conjure_effect(3175, None),
                                 ('CONST_ME_MAGIC_RED' if next_is_rune else None, next_is_rune))


class ConjureQuantityReferenceTests(unittest.TestCase):
    def candidate(self, name='arrow call'):
        resolution = CONJURE_RESOLUTIONS['spells']['arrow call']
        page = copy.deepcopy(resolution['source'])
        wikis = Wikis({'api': page['api'], 'pages': [page]}, {'pages': []}, {'changes': []})
        records = {'canary': {'spell_type': 'instant', 'file': 'arrow.lua',
                   'registrar': {'name': 'Arrow Call', 'words': 'exevo infir con', 'id': 176}}}
        return Bundle(name, records, wikis, {}, {('canary', 'arrow.lua'): ''})

    def test_exact_historical_quantity_has_mediawiki_provenance(self):
        bundle = self.candidate()
        self.assertEqual(bundle.conjure_quantity_reference(3, {'result_item_id': 21470, 'count': 3}), 30)
        row = next(r for r in bundle.rows if r.get('destination'))
        self.assertEqual(row['destination'], '/spell/spell/execution/conjure/count')
        source = bundle.sources[row['source_index']]
        self.assertEqual((source['kind'], source['revision_id']), ('mediawiki', 1182610))
        self.assertIn('Quantity is distinct from mana', row['resolution'])

    def test_conjure_arrow_and_rune_counts_are_not_overridden(self):
        self.assertEqual(self.candidate('conjure arrow').conjure_quantity_reference(
            10, {'result_item_id': 3447, 'count': 10}), 10)
        self.assertEqual(self.candidate('intense healing rune').conjure_quantity_reference(
            1, {'result_item_id': 3160, 'count': 1}), 1)

    def test_wrong_identity_fails_before_quantity_resolution(self):
        for field, value in [('words', 'exevo con'), ('id', 51), ('name', 'Conjure Arrow')]:
            bundle = self.candidate()
            bundle.records['canary']['registrar'][field] = value
            with self.subTest(field=field), self.assertRaises(Unresolved):
                bundle.conjure_quantity_reference(3, {'result_item_id': 21470, 'count': 3})
        bundle = self.candidate()
        bundle.records['canary']['spell_type'] = 'rune'
        with self.assertRaises(Unresolved):
            bundle.conjure_quantity_reference(3, {'result_item_id': 21470, 'count': 3})
        with self.assertRaises(Unresolved):
            self.candidate().conjure_quantity_reference(3, {'result_item_id': 3447, 'count': 3})

    def test_unexpected_previous_or_donor_count_fails_closed(self):
        for previous, count in [(4, 3), (3, 4), (True, 3), (3, True)]:
            with self.subTest(previous=previous, count=count), self.assertRaises(Unresolved):
                self.candidate().conjure_quantity_reference(previous, {'result_item_id': 21470, 'count': count})

    def test_historical_source_identity_and_digest_are_fenced(self):
        for field, value in [('page_id', 1), ('revision_id', 1), ('content_sha256', '0' * 64)]:
            bundle = self.candidate()
            bundle.wikis.docs['fandom']['pages'][0][field] = value
            with self.subTest(field=field), self.assertRaises(Unresolved):
                bundle.conjure_quantity_reference(3, {'result_item_id': 21470, 'count': 3})


class CurrentRuneUseReferenceTests(unittest.TestCase):
    def candidate(self, name, carrier='rune'):
        records = {'canary': {'spell_type': carrier, 'file': 'test.lua', 'registrar': {
            'vocation': ['druid'], 'group': 'support', 'groupCooldown': 2000, 'amount': 3}}}
        wikis = Wikis({'pages': [], 'api': 'https://tibia.fandom.com/api.php',
                       'target_cut': '2026-09-27'}, {'pages': []}, {'changes': []})
        bundle = Bundle(name, records, wikis, {}, {('canary', 'test.lua'): ''})
        page = {'title': name, 'page_id': 1, 'revision_id': 2, 'content_sha256': 'a' * 64,
                'timestamp': '2026-09-01', 'fields': {'voc': 'druid',
                    'vocrequired': 'druid, knight, paladin, sorcerer',
                    'runegroup': 'support', 'subclass': 'support', 'cooldowngroup': '2'}}
        return bundle, {'fandom': page}

    def assert_manifest(self, bundle, field, destination):
        schema = json.loads((MONSTER / 'monster-import-readiness.schema.json').read_text(encoding='utf-8'))
        manifest = {'sources': bundle.sources, 'entries': bundle.rows}
        self.assertEqual(list(Draft202012Validator(schema).iter_errors(manifest)), [])
        current = [r for r in bundle.rows if r.get('destination') == destination]
        self.assertEqual(len(current), 1)
        source = bundle.sources[current[0]['source_index']]
        expected = RUNE_USE_RESOLUTIONS['spells'][bundle.name]
        self.assertEqual(source, expected['source'])
        self.assertEqual(source['kind'], 'community_capture')
        self.assertEqual(source['captured'], '2026-10-09')
        self.assertEqual(current[0]['source_field'], expected['source_field'])
        self.assertIn(expected['client_proof']['sha256'], current[0]['resolution'])
        self.assertIn('third-party mirror/API', current[0]['resolution'])
        self.assertIn(expected['origin_kind'], current[0]['resolution'])
        self.assertIn(expected['canonical_tibia_url'], current[0]['resolution'])
        self.assertIn('not pre-cut evidence or runtime admission', current[0]['resolution'])
        historical = [r for r in bundle.rows if r['source_field'] == field]
        self.assertEqual(len(historical), 1)
        self.assertEqual(historical[0]['status'], 'approved_omission')
        self.assertNotIn('destination', historical[0])
        self.assertEqual(bundle.sources[historical[0]['source_index']]['revision_id'], 2)
        self.assertIn('S3:', historical[0]['resolution'])

    def test_ihr_use_adds_monk_and_promotion_without_rewriting_wiki(self):
        bundle, pages = self.candidate('intense healing rune')
        old_pages, old_records = copy.deepcopy(pages), copy.deepcopy(bundle.records)
        self.assertEqual(bundle.vocations(pages, 'rune'),
                         RUNE_USE_RESOLUTIONS['spells'][bundle.name]['value'])
        self.assert_manifest(bundle, 'vocrequired', '/spell/spell/requirements/vocations')
        self.assertEqual(pages, old_pages)
        self.assertEqual(bundle.records, old_records)
        self.assertEqual(bundle.wikis.resolve(pages, 'vocrequired', bundle.name)[0],
                         ['druid', 'knight', 'paladin', 'sorcerer'])

    def test_paralyse_use_changes_only_primary_group(self):
        bundle, pages = self.candidate('paralyze rune')
        pages['fandom']['fields'].update(secondarygroup='special', cooldowngroup2='1')
        old_pages = copy.deepcopy(pages)
        self.assertEqual(bundle.groups(pages, 'rune'),
                         [{'group': 'attack', 'cooldown_ms': 2000},
                          {'group': 'special', 'cooldown_ms': 1000}])
        self.assert_manifest(bundle, 'runegroup', '/spell/spell/groups/0/group')
        self.assertEqual(pages, old_pages)
        self.assertEqual(bundle.wikis.resolve(pages, 'runegroup', bundle.name)[0], 'support')

    def test_instant_conjuring_keeps_original_vocations_and_group(self):
        for name in ('intense healing rune', 'paralyze rune'):
            with self.subTest(name=name):
                bundle, pages = self.candidate(name, 'instant')
                self.assertEqual(bundle.vocations(pages, 'instant'), ['druid', 'elder_druid'])
                self.assertEqual(bundle.groups(pages, 'instant'),
                                 [{'group': 'support', 'cooldown_ms': 2000}])
                self.assertFalse(any(s.get('kind') == 'community_capture' for s in bundle.sources))

    def test_wrong_source_carrier_cannot_consume_use_reference(self):
        bundle, _ = self.candidate('intense healing rune', 'instant')
        previous = RUNE_USE_RESOLUTIONS['spells'][bundle.name]['previous']
        self.assertEqual(bundle.current_rune_use_reference('vocations', 'rune', previous,
                         '/spell/spell/requirements/vocations'), previous)
        self.assertEqual(bundle.rows, [])

    def test_unexpected_previous_value_requires_new_disposition(self):
        bundle, _ = self.candidate('paralyze rune')
        with self.assertRaises(Unresolved):
            bundle.current_rune_use_reference('primary_group', 'rune', 'healing',
                                               '/spell/spell/groups/0/group')
        self.assertEqual(bundle.rows, [])

    def test_arrow_call_quantity_and_unrelated_source_fields_stay_unchanged(self):
        self.assertEqual(set(RUNE_USE_RESOLUTIONS['spells']),
                         {'intense healing rune', 'paralyze rune'})
        bundle, pages = self.candidate('arrow call', 'instant')
        previous = copy.deepcopy(bundle.records)
        self.assertEqual(bundle.groups(pages, 'instant'),
                         [{'group': 'support', 'cooldown_ms': 2000}])
        self.assertEqual(bundle.records, previous)
        self.assertEqual(bundle.records['canary']['registrar']['amount'], 3)
        self.assertFalse(any(s.get('kind') == 'community_capture' for s in bundle.sources))


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
