"""Companion source closure, condition math and ordered acquisition refusals."""
import copy
import json
import os
from pathlib import Path
import unittest

from jsonschema import Draft202012Validator
import native_companions as nc

SOURCE_ROOT = Path(os.environ.get('OTERYN_SPELL_SOURCES', '/workspace/spell-sources'))
ROOT = Path(__file__).resolve().parent


class MechanicsTests(unittest.TestCase):
    def recipe(self, name, source='canary'):
        return nc._recipe(name, source)['parameters']

    def test_caster_offsets_follow_float32_formula_then_truncation(self):
        # Explicit source coefficient arithmetic at base400; Haste's float32
        # 1.3 product is below468, so truncation yields delta107, not108.
        for name, expected in [('haste', 107), ('strong haste', 252),
                               ('charge', 324), ('swift foot', 320)]:
            with self.subTest(name=name):
                self.assertEqual(nc.caster_speed_delta(self.recipe(name), 400), expected)

    def test_familiar_uses_base_speeds_not_owner_current_or_caster_offset(self):
        for name, expected in [('haste', 96), ('strong haste', 224),
                               ('charge', 288), ('swift foot', 248)]:
            self.assertEqual(nc.familiar_speed_delta(self.recipe(name), 400, 220), expected)
        self.assertEqual(nc.familiar_speed_delta(self.recipe('haste'), 80, 80), 0)
        self.assertEqual(self.recipe('haste')['familiar']['zero_delta_paralyze_total'], 40)

    def test_caster_and_familiar_haste_durations_are_distinct(self):
        p = self.recipe('haste')
        self.assertEqual((p['caster']['duration_ms'], p['familiar']['duration_ms']), (30000, 33000))
        self.assertEqual(p['execution_order'], ['familiar_conditions', 'caster_combat'])
        self.assertFalse(p['ordinary_summons_receive_condition'])
        self.assertFalse(p['replacement']['stronger_wins'])
        self.assertTrue(p['replacement']['preserve_infinite_from_timed'])

    def test_swift_modifier_after_success_and_source_conflict_preserved(self):
        ca, cr = self.recipe('swift foot'), self.recipe('swift foot', 'crystal')
        self.assertEqual(ca['damage_dealt_percent'], {'none': 70, 'regular': 70, 'greater': 70})
        self.assertEqual(cr['damage_dealt_percent'], {'none': 70, 'regular': 50, 'greater': 100})
        self.assertEqual(ca['execution_order'][-1], 'swift_damage_modifier')
        self.assertTrue(ca['damage_modifier_after_combat_success'])
        self.assertTrue(ca['attacks_and_casts_allowed'])

    def test_swift_attribute_slot_and_crystal_cast_only_party_registration(self):
        slot = self.recipe('swift foot')['damage_modifier_condition']
        self.assertEqual((slot['type'], slot['id'], slot['sub_id'], slot['buff']),
                         ('attributes', 'combat', 0, False))
        self.assertTrue(slot['preserve_longer_remaining'])
        self.assertEqual(slot['refresh'], 'end_old_effects_then_replace_entire_slot')
        ca = self.recipe('knight familiar')['party_protection_registration']
        cr = self.recipe('summon knight familiar', 'crystal')['party_protection_registration']
        self.assertEqual(ca['event'], 'never')
        self.assertEqual(cr['event'], 'cast_success')
        self.assertEqual(cr['target'], 'all_owned_summons')
        self.assertTrue(self.recipe('knight familiar')['cooldown']['preserve_on_owner_death'])

    def test_lifetime_cooldown_config_units_and_vip_order(self):
        p = self.recipe('knight familiar')
        self.assertEqual(nc.familiar_times(p), (900000, 1800000))
        self.assertEqual(nc.familiar_times(p, vip=True, vip_reduction_minutes=10), (900000, 1200000))
        self.assertEqual(nc.familiar_times(p, config_minutes=10, rate_divisor=2), (300000, 300000))
        # Preserve the source seconds-vs-minutes cap rather than silently fix it.
        self.assertEqual(nc.familiar_times(p, vip=True, vip_reduction_minutes=20), (900000, 600000))
        for invalid in (0, -1, float('nan'), float('inf')):
            with self.assertRaises(ValueError):
                nc.familiar_times(p, rate_divisor=invalid)

    def test_canary_runs_offline_crystal_pauses_and_expired_does_not_restore(self):
        ca, cr = self.recipe('monk familiar'), self.recipe('monk familiar', 'crystal')
        self.assertEqual(nc.login_remaining(ca, 900, 3900, 300), 0)
        self.assertEqual(nc.login_remaining(cr, 900, 3900, 300), 600)
        self.assertEqual(nc.login_remaining(cr, 900, 3900, 1000), -100)
        self.assertTrue(cr['login']['recreate_if_remaining_positive'])
        self.assertEqual(ca['shared_cooldown_identity'], cr['shared_cooldown_identity'])

    def test_short_warning_unsigned_conversion_then_scheduler_minimum(self):
        result = nc.warning_schedule(self.recipe('knight familiar'), 30000)
        self.assertEqual([r['raw_delay_ms'] for r in result], [20000, -30000])
        self.assertEqual([r['delay_ms'] for r in result], [20000, 100])
        self.assertEqual(result[1]['message'], 'Your summon will disappear in less than one minute')
        for source in ('canary', 'crystal'):
            for file in ('src/lua/functions/core/game/global_functions.cpp',
                         'src/lua/functions/lua_functions_loader.hpp'):
                self.assertIn(file, nc.HELPER_HASHES[source])
                self.assertIn(file, nc._helper_paths('knight familiar'))

    def test_warning_order_exact_and_death_does_not_reset_cooldown(self):
        p = self.recipe('druid familiar')
        self.assertEqual([w['remaining_ms']for w in p['warnings']], [10000, 60000])
        self.assertFalse(p['familiar_death']['reset_spell_cooldown'])
        self.assertFalse(p['manual_dispel']['clear_saved_expiry'])
        self.assertTrue(p['expiry']['requires_owner_and_creature_present'])

    def test_return_boundary_floor_pz_and_teleport_exception(self):
        p = self.recipe('paladin familiar')
        self.assertFalse(nc.must_return(p, 15, -15, 0))
        self.assertTrue(nc.must_return(p, 16, 0, 0))
        self.assertTrue(nc.must_return(p, 0, 0, 1))
        self.assertFalse(nc.must_return(p, 30, 0, 1, True))
        self.assertFalse(p['return_to_owner']['protection_zone_exclusion'])

    def named_facts(self, **updates):
        return dict(type_found=True, summonable=True, owned_summons=1, creature_mana_cost=620,
                    mana=620, has_infinite_mana=False, spawn_room=True, **updates)

    def test_named_cap_refusal_preserves_cost_and_world(self):
        facts = self.named_facts()
        facts['owned_summons'] = 2
        before = copy.deepcopy(facts)
        result = nc.acquisition_admission(self.recipe('summon creature'), facts)
        self.assertEqual(result, {'accepted': False, 'reason': 'You cannot summon more creatures.'})
        self.assertEqual(facts, before)

    def test_summon_all_bypasses_flags_and_cap_but_not_missing_type_or_mana(self):
        p, facts = self.recipe('summon creature'), self.named_facts()
        facts.update(can_summon_all=True, summonable=False, owned_summons=5, mana=1)
        self.assertEqual(nc.acquisition_admission(p, facts)['reason'], 'not_enough_mana')
        facts['has_infinite_mana'] = True
        result = nc.acquisition_admission(p, facts)
        self.assertTrue(result['accepted'])
        self.assertEqual(result['mana_cost'], 620)
        self.assertFalse(result['consume_rune_charge'])
        self.assertEqual(result['commit_order'][0], 'create_owned_creature')
        facts['type_found'] = False
        self.assertEqual(nc.acquisition_admission(p, facts)['reason'], 'not_possible')

    def test_room_refusal_precedes_spending(self):
        facts = self.named_facts()
        facts['spawn_room'] = False
        result = nc.acquisition_admission(self.recipe('summon creature'), facts)
        self.assertEqual(result, {'accepted': False, 'reason': 'not_enough_room'})
        self.assertNotIn('mana_cost', result)

    def corpse_facts(self):
        return dict(tile_present=True, top_down_item_present=True, is_corpse=True,
                    movable=True, owned_summons=1, black_skull=False, spawn_room=True)

    def test_animate_dead_requires_actual_movable_corpse_and_cannot_bypass_cap(self):
        p = self.recipe('animate dead rune')
        for field, value in [('tile_present', False), ('is_corpse', False), ('movable', False),
                             ('spawn_room', False), ('black_skull', True), ('owned_summons', 2)]:
            with self.subTest(field=field):
                facts = self.corpse_facts()
                facts[field] = value
                facts.update(can_summon_all=True, can_convince_all=True, none=True)
                result = nc.acquisition_admission(p, facts)
                self.assertFalse(result['accepted'])
                self.assertNotIn('consume_rune_charge', result)
        result = nc.acquisition_admission(p, self.corpse_facts())
        self.assertEqual(result['mana_cost'], 0)
        self.assertTrue(result['consume_rune_charge'])
        self.assertEqual(result['commit_order'][:3], ['create_skeleton', 'remove_corpse', 'assign_master'])

    def convince_facts(self):
        return dict(target_is_monster=True, convinceable=True, master_name=None,
                    owned_summons=1, creature_mana_cost=300, mana=300, has_infinite_mana=False)

    def test_convince_master_rule_carved_tile_case_and_override(self):
        p, facts = self.recipe('convince creature rune'), self.convince_facts()
        facts['master_name'] = 'Some Player'
        self.assertEqual(nc.acquisition_admission(p, facts)['reason'], 'not_possible')
        facts['master_name'] = 'A CARVED STONE TILE'
        result = nc.acquisition_admission(p, facts)
        self.assertTrue(result['accepted'])
        self.assertEqual(result['commit_order'], ['subtract_mana', 'add_mana_spent', 'assign_master', 'caster_magic_blue'])
        facts.update(can_convince_all=True, master_name='Some Player', owned_summons=8, convinceable=False)
        self.assertTrue(nc.acquisition_admission(p, facts)['accepted'])
        facts['target_is_monster'] = False
        self.assertEqual(nc.acquisition_admission(p, facts)['reason'], 'not_possible')

    def test_closed_schemas_reject_missing_unknown_and_semantically_changed_parameters(self):
        schemas = nc.schemas()
        for name, spec in nc.SPECS.items():
            for source in spec['sources']:
                recipe = nc._recipe(name, source)
                validator = Draft202012Validator(schemas[recipe['key']])
                self.assertFalse(list(validator.iter_errors(recipe['parameters'])))
                changed = copy.deepcopy(recipe['parameters'])
                changed['lua_body'] = 'return true'
                self.assertTrue(list(validator.iter_errors(changed)))
                changed = copy.deepcopy(recipe['parameters'])
                del changed[next(iter(changed))]
                self.assertTrue(list(validator.iter_errors(changed)))
        p = self.recipe('summon creature')
        p['summon_cap'] = 99
        self.assertTrue(list(Draft202012Validator(schemas['acquire_summon']).iter_errors(p)))

    def test_unknown_name_or_wrong_carrier_has_no_fabricated_native_behavior(self):
        self.assertIsNone(nc.build('unknown', 'instant', {}, {}))
        self.assertIsNone(nc.build('animate dead rune', 'instant', {}, {}))
        with self.assertRaises(ValueError):
            nc.build('haste', 'instant', {}, {})


@unittest.skipUnless((SOURCE_ROOT / 'canary/data/libs/functions/player.lua').exists() and
                     (SOURCE_ROOT / 'crystal/data/libs/functions/player.lua').exists(),
                     'Exact source integration fixtures are not installed; pure mechanics tests remain independent.')
class ExactSourceIntegrationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.census = json.loads((ROOT / 'samples/spell-census-canary-99902524-crystal-ff7ede5.json').read_text())

    def inputs(self, name):
        records, texts = {}, {}
        for source in nc.SPECS[name]['sources']:
            record = next(r for r in self.census[source]
                          if r['name'].casefold() == name and r['spell_type'] == nc.SPECS[name]['carrier'])
            records[source] = {**record, 'source_root': SOURCE_ROOT / source}
            texts[source, record['file']] = (SOURCE_ROOT / source / record['file']).read_text()
        return records, texts

    def test_all_sixteen_records_sourcequalify_and_keep_strict_parameters(self):
        self.assertEqual(len(nc.SPECS), 16)
        schema = nc.schemas()
        for name, spec in nc.SPECS.items():
            with self.subTest(name=name):
                records, texts = self.inputs(name)
                recipe = nc.build(name, spec['carrier'], records, texts)
                self.assertEqual(set(recipe), {'key', 'parameters'})
                Draft202012Validator(schema[recipe['key']]).validate(recipe['parameters'])
                proof = nc.evidence(name, spec['carrier'], records, texts)
                self.assertEqual(proof['selection'], 'canary' if 'canary' in records else 'crystal')
                self.assertTrue(all(len(p['files']) > 1 for p in proof['sources']))

    def test_changed_script_census_hash_revision_or_helper_fails_closed(self):
        records, texts = self.inputs('haste')
        for mode in ('script', 'census', 'revision', 'helper'):
            with self.subTest(mode=mode):
                changed_records, changed_texts = copy.deepcopy(records), copy.deepcopy(texts)
                if mode == 'script':
                    key = ('canary', records['canary']['file'])
                    changed_texts[key] += '\ncreature:addItem(1)\n'
                elif mode == 'census':
                    changed_records['canary']['blob'] = '0' * 40
                elif mode == 'revision':
                    changed_records['canary']['revision'] = '0' * 40
                else:
                    changed_texts['canary', 'src/creatures/combat/condition.cpp'] = 'changed helper'
                with self.assertRaises(ValueError):
                    nc.build('haste', 'instant', changed_records, changed_texts)

    def test_missing_helper_cannot_be_masked_by_a_known_cast_body(self):
        records, texts = self.inputs('knight familiar')
        del records['canary']['source_root']
        with self.assertRaisesRegex(ValueError, 'missing pinned helper'):
            nc.build('knight familiar', 'instant', records, texts)

    def test_alias_reference_identity_and_distinct_login_provenance(self):
        ca = nc.build('knight familiar', 'instant', *self.inputs('knight familiar'))
        cr = nc.build('summon knight familiar', 'instant', *self.inputs('summon knight familiar'))
        self.assertEqual(ca['parameters']['reference_spell_id'], cr['parameters']['reference_spell_id'])
        self.assertEqual(ca['parameters']['shared_cooldown_identity'], cr['parameters']['shared_cooldown_identity'])
        self.assertNotEqual(ca['parameters']['login'], cr['parameters']['login'])
        self.assertIn('aliases share reference ID', ' '.join(
            nc.evidence('summon knight familiar', 'instant', *self.inputs('summon knight familiar'))['conflicts']))


if __name__ == '__main__':
    unittest.main()
