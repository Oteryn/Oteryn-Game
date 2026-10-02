"""Closed authoring ABI, hostile provenance and independently stated source facts."""
from copy import deepcopy
import hashlib
from pathlib import Path
import unittest
from unittest.mock import patch

from jsonschema import Draft202012Validator, ValidationError

import native_actor_states as states
from validate_spell import evaluate


NAMES = {
    'focus harmony', 'focus serenity', 'virtue of harmony', 'virtue of justice',
    'virtue of sustain', 'blood rage', 'protector', 'sharpshooter',
    'shield bash', 'shield slam', 'flurry of blows', 'sweeping takedown',
}


def synthetic_source(name='focus harmony'):
    """Own synthetic bytes exercise the hash gate without redistributing Lua."""
    text = 'local audit_fixture = 1\n'
    data = text.encode()
    spec = {'file': 'data/scripts/spells/support/focus_harmony.lua',
            'sha256': hashlib.sha256(data).hexdigest(),
            'blob': hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest()}
    records = {'canary': {'name': name, 'spell_type': 'instant', 'file': spec['file'],
                          'blob': spec['blob'], 'source_root': Path('/owned/fixture'),
                          'revision': states.PINS['canary']}}
    return spec, records, {('canary', spec['file']): text}


class ActorStateAuthoringTests(unittest.TestCase):
    def test_exact_twelve_records_and_strict_finite_schemas(self):
        self.assertEqual(set(states.MODELS), NAMES)
        self.assertEqual(set(states.SOURCE_SPECS), NAMES)
        schemas = states.schemas()
        for name, model in states.MODELS.items():
            with self.subTest(name=name):
                Draft202012Validator.check_schema(schemas[model['key']])
                validator = Draft202012Validator(schemas[model['key']])
                validator.validate(model['parameters'])
                for field in model['parameters']:
                    invalid = deepcopy(model['parameters']); invalid.pop(field)
                    with self.assertRaises(ValidationError):
                        validator.validate(invalid)
                invalid = deepcopy(model['parameters']); invalid['script'] = 'opaque Lua'
                with self.assertRaises(ValidationError):
                    validator.validate(invalid)

    def test_build_returns_fresh_native_payload_and_exact_names_only(self):
        with patch.object(states, '_qualified', return_value=True):
            first = states.build('Focus Harmony', 'instant', {}, {})
            self.assertEqual(set(first), {'key', 'parameters'})
            first['parameters']['harmony_max'] = 100
            self.assertEqual(states.build('focus harmony', 'instant', {}, {})['parameters']['harmony_max'], 5)
            for name, kind in [('focus_harmony', 'instant'), ('focus harmony ', 'instant'),
                               ('focus harmony', 'rune'), ('unknown spell', 'instant'),
                               (None, 'instant')]:
                self.assertIsNone(states.build(name, kind, {}, {}))

    def test_full_file_hash_blob_path_kind_name_head_and_revision_fail_closed(self):
        spec, records, texts = synthetic_source()
        with patch.object(states, 'SOURCE_SPECS', {'focus harmony': {'canary': spec}}), \
             patch.object(states, 'HELPERS', {}), \
             patch.object(states, '_head', return_value=states.PINS['canary']):
            self.assertIsNotNone(states.build('focus harmony', 'instant', records, texts))
            altered = {k: v + '-- modified\n' for k, v in texts.items()}
            self.assertIsNone(states.build('focus harmony', 'instant', records, altered))
            self.assertIsNone(states.build('focus harmony', 'instant', records, {}))
            for key, value in [('name', 'focus serenity'), ('name', None), ('spell_type', 'rune'),
                               ('file', 'different.lua'), ('blob', '0' * 40),
                               ('revision', '0' * 40), ('source_root', None)]:
                bad = deepcopy(records); bad['canary'][key] = value
                self.assertIsNone(states.build('focus harmony', 'instant', bad, texts))
            with patch.object(states, '_head', return_value='0' * 40):
                self.assertIsNone(states.build('focus harmony', 'instant', records, texts))
            self.assertIsNone(states.build('focus harmony', 'instant', {}, texts))

    def test_missing_or_changed_selected_engine_helper_cannot_qualify(self):
        spec, records, texts = synthetic_source()
        helper = 'src/creatures/players/player.cpp'
        data = b'own deterministic helper fixture\n'
        digest = hashlib.sha256(data).hexdigest()
        with patch.object(states, 'SOURCE_SPECS', {'focus harmony': {'canary': spec}}), \
             patch.object(states, 'HELPERS', {helper: digest}), \
             patch.object(states, '_head', return_value=states.PINS['canary']):
            self.assertIsNone(states.build('focus harmony', 'instant', records, texts))
            texts[('canary', helper)] = data.decode()
            self.assertIsNotNone(states.build('focus harmony', 'instant', records, texts))
            texts[('canary', helper)] += 'changed'
            self.assertIsNone(states.build('focus harmony', 'instant', records, texts))

    def test_focus_full_fill_healing_and_forced_window_have_concrete_data(self):
        harmony = states.MODELS['focus harmony']['parameters']
        serene = states.MODELS['focus serenity']['parameters']
        self.assertEqual((harmony['harmony_max'], harmony['serene_ms']), (5, None))
        self.assertEqual((serene['harmony_max'], serene['serene_ms']), (5, 7000))
        self.assertTrue(serene['forced_serene_prevents_automatic_clear'])
        self.assertFalse(harmony['reset_spender_cooldowns'])
        self.assertTrue(serene['reset_spender_cooldowns'])
        self.assertEqual(serene['cooldown_reset_filter']['individual'], 'spender_only')
        self.assertEqual(serene['cooldown_reset_filter']['group_without_resolved_spell'], 'clear')
        self.assertEqual(serene['cooldown_reset_filter']['group_with_resolved_spell'], 'spender_only')
        heal = serene['harmony_gain_healing']
        self.assertEqual(heal['when'], 'positive_gained_charges_only')
        self.assertEqual(heal['sustain_application'], 'after_world_healing_roll')
        self.assertEqual(heal['sustain_quantization'], 'truncate_toward_zero')
        # S5 F(200)=40. Two newly gained points ->ceil88,ceil101.2.
        env = {'level': 200, 'gained_charges': 2}
        self.assertEqual([evaluate(heal['bounds'][k], env) for k in ('minimum', 'maximum')], [88, 102])
        self.assertEqual([evaluate(heal['bounds'][k], {'level': 1, 'gained_charges': 5})
                          for k in ('minimum', 'maximum')], [10, 25])
        self.assertEqual(heal['target_selection'], 'lowest_absolute_health_from_self_and_visible_party')

    def test_stances_retain_d145_and_final_skill_scope_with_base_fist_exception(self):
        for name in ('blood rage', 'protector', 'sharpshooter',
                     'virtue of harmony', 'virtue of justice', 'virtue of sustain'):
            params = states.MODELS[name]['parameters']
            self.assertEqual(params['slot'], 'standard')
            self.assertTrue(params['toggle_same_stance_off'])
            self.assertTrue(params['replace_existing'])
            self.assertTrue(params['persist_across_sessions'])
            self.assertTrue(params['keep_on_death'])
        rage = states.MODELS['blood rage']['parameters']['modifiers']
        self.assertEqual(states.MODELS['blood rage']['parameters']['eligible_vocations'], ['elite_knight'])
        self.assertEqual(states.MODELS['sharpshooter']['parameters']['eligible_vocations'], ['royal_paladin'])
        self.assertEqual(rage[0]['skills'], ['sword', 'axe', 'club'])
        self.assertEqual(rage[0]['percent'], 25)
        self.assertEqual(rage[1]['percent'], 15)
        sharp = states.MODELS['sharpshooter']['parameters']['modifiers'][0]
        self.assertEqual(sharp['percent'], 32)
        self.assertEqual(sharp['skills'], ['distance'])
        justice = states.MODELS['virtue of justice']['parameters']['modifiers'][0]
        self.assertEqual((justice['percent'], justice['serene_percent'], justice['basis']), (8, 16, 'base'))
        harmony = states.MODELS['virtue of harmony']['parameters']['modifiers'][0]
        self.assertEqual((harmony['percent'], harmony['serene_percent'], harmony['spender_refund_charges']), (50, 100, 1))
        self.assertNotIn('ascetic', str(states.MODELS).lower())
        self.assertNotIn('base_power_harmony', str(states.MODELS).lower())

    def test_shield_attack_real_base_inputs_and_one_use_debuff(self):
        for name, power, average in [('shield bash', 55, 110), ('shield slam', 52, 104)]:
            params = states.MODELS[name]['parameters']
            env = {'level': 1, 'base_power': power, 'shielding_skill': 100, 'shield_defense': 20}
            self.assertEqual([evaluate(params['formula'][k], env) for k in ('minimum', 'maximum')],
                             [average * 0.9, average * 1.1])
            self.assertEqual(params['input']['hand_order'], ['left', 'right'])
            self.assertEqual(params['input']['selection'], 'first_shield')
            debuff = params['next_auto_attack_reduction']
            self.assertEqual((debuff['duration_ms'], debuff['percent']), (10000, 50))
            self.assertEqual(debuff['origins'], ['melee', 'ranged', 'fist'])
            self.assertFalse(debuff['consume_on_spell'])
            self.assertTrue(debuff['affects_primary_and_secondary'])
            self.assertEqual(debuff['consume_at'], 'first_auto_attack_damage_step')
        self.assertEqual(states.MODELS['shield slam']['parameters']['area']['rows'], ['xxx', 'xcx', 'xxx'])
        self.assertEqual(states.MODELS['shield slam']['parameters']['next_auto_attack_reduction']['wheel_grade_2_additional_percent'], 25)

    def test_monk_directional_areas_and_sweeping_cache_are_not_shared_random_draws(self):
        flurry = states.MODELS['flurry of blows']['parameters']
        sweep = states.MODELS['sweeping takedown']['parameters']
        self.assertEqual(sum(r.count('x') + r.count('C') for r in flurry['area']['rows']), 9)
        self.assertEqual(sum(r.count('x') + r.count('C') for r in flurry['wheel_enlarged_area']['rows']), 14)
        self.assertEqual(sum(r.count('x') + r.count('C') for r in sweep['area']['rows']), 12)
        self.assertEqual(sum(r.count('x') for r in sweep['outer_area']['rows']), 10)
        self.assertEqual(sweep['outer_area']['factor'], '0.75')
        self.assertEqual(sweep['outer_area']['draw'], 'separate_world_roll')
        self.assertEqual(sweep['harmony_scaling'], 'existing_whole_bound_multiplier_once')
        self.assertEqual(sweep['cast_cache'], 'occurrence_local_bounds_not_global_caster_cache')
        # Source callbacks: skill140,attack46,P48,F(1000)=183,
        # skillBonus=(140-110)^2*.026=23.4 =>center515.52.
        env = {'level': 1000, 'attack_skill': 140, 'attack_value': 46,
               'base_power': 48, 'sweeping_skill_bonus': 23.4}
        self.assertAlmostEqual(evaluate(sweep['formula']['minimum'], env), 670.176)
        self.assertAlmostEqual(evaluate(sweep['formula']['maximum'], env), 876.384)
        self.assertEqual(sweep['skill_bonus']['steps'][0], {'above': 250, 'coefficient': '0.043'})
        self.assertEqual(sweep['skill_bonus']['steps'][-1], {'above': 110, 'coefficient': '0.022'})

    def test_evidence_does_not_claim_runtime_admission_and_preserves_conflicts(self):
        evidence = states.evidence()
        self.assertEqual(set(evidence), NAMES)
        for row in evidence.values():
            self.assertFalse(row['runtime_ready'])
            self.assertIn('D145', row['stance_retention'])
            self.assertEqual(set(row['source_files']), {'canary', 'crystal'})
            self.assertTrue(row['conflicts'])

    def test_available_real_pinned_checkouts_qualify_all_twelve(self):
        root = Path('/workspace/spell-sources')
        if not (root / 'canary/.git').exists() or not (root / 'crystal/.git').exists():
            self.skipTest('Optional full-file source integration check requires pinned source checkouts')
        for name, sources in states.SOURCE_SPECS.items():
            records, texts = {}, {}
            for source, spec in sources.items():
                records[source] = {'name': name, 'spell_type': 'instant', **spec,
                                   'source_root': root / source, 'revision': states.PINS[source]}
                texts[(source, spec['file'])] = (root / source / spec['file']).read_text()
            with self.subTest(name=name):
                self.assertIsNotNone(states.build(name, 'instant', records, texts))


if __name__ == '__main__':
    unittest.main()
