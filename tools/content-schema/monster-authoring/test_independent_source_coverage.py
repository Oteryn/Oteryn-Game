"""Regression mutations for source coverage, independent of converter transforms."""
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import verify_monster_source_values as values
import verify_source_coverage as registrar

SOURCE = '''local mType=Game.createMonsterType("Example")
local monster={}
monster.maxHealth=100
monster.health=100
monster.flags={attackable=false,summonable=false,convinceable=false}
mType:register(monster)
'''


class SourceCoverageTests(unittest.TestCase):
    def setUp(self):
        self.monster = {'creature': {'stats': {'max_health': 100, 'initial_health': 100},
                                    'flags': {'attackable': False},
                                    'summoning': {'summonable': False, 'convinceable': False}}}
        self.manifest = {'sources': [{}], 'entries': [
            {'source_index': 0, 'source_file': 'example.lua', 'source_field': 'flags',
             'status': 'mapped', 'destination': '/monster/creature/flags'}]}
        self.raw = values.evaluate(SOURCE)[0][0]['leaves']

    def check(self):
        return values.check_values(self.raw, self.monster, self.manifest, SOURCE, 'example.lua', 0)

    def test_independent_value_mutation_and_drop(self):
        self.assertFalse(self.check()[1])
        self.monster['creature']['stats']['max_health'] = 99
        self.assertIn('maxHealth: SOURCE_VALUE_MISMATCH', self.check()[1])
        del self.monster['creature']['stats']['initial_health']
        self.assertIn('health: SOURCE_VALUE_DROPPED', self.check()[1])

    def test_parent_manifest_does_not_prove_leaf_preservation(self):
        del self.monster['creature']['flags']['attackable']
        self.assertIn('flags.attackable: SOURCE_VALUE_DROPPED', self.check()[1])

    def test_undeclared_source_field_is_not_covered(self):
        self.raw['futureMechanic.potency'] = 50
        rows, errors = self.check()
        row = next(r for r in rows if r['source_field'] == 'futureMechanic.potency')
        self.assertEqual(row['status'], 'UNACCOUNTED_SOURCE_FIELD')
        self.assertFalse(errors)

    def test_nonexistent_declared_destination(self):
        self.manifest['entries'][0]['destination'] = '/monster/creature/flags/missing'
        self.assertTrue(values.check_manifest(self.monster, {}, self.manifest))

    def test_event_name_disposition_must_bind_exact_observed_value(self):
        self.raw['events[1]'] = 'ObservedEvent'
        self.manifest['entries'].append({'source_index': 0, 'source_file': 'example.lua', 'source_field': 'events=OtherEvent', 'status': 'approved_omission'})
        rows, _ = self.check()
        self.assertEqual(next(r for r in rows if r['source_field'] == 'events[1]')['status'], 'UNACCOUNTED_SOURCE_FIELD')
        self.manifest['entries'][-1]['source_field'] = 'events=ObservedEvent'
        rows, _ = self.check()
        self.assertEqual(next(r for r in rows if r['source_field'] == 'events[1]')['status'], 'UNKNOWN_DOCUMENTED_SOURCE_OMISSION')

    def test_boolean_zero_is_not_preservation(self):
        self.monster['creature']['flags']['attackable'] = 0
        self.assertIn('flags.attackable: SOURCE_VALUE_MISMATCH', self.check()[1])

    def test_missing_mana_unspecified_vs_disabled(self):
        rows, _ = self.check()
        self.assertEqual(next(r for r in rows if r['source_field'] == 'manaCost')['status'], 'NOT_APPLICABLE_IN_SOURCE')
        del self.raw['flags.summonable']
        rows, _ = self.check()
        self.assertEqual(next(r for r in rows if r['source_field'] == 'manaCost')['status'], 'SOURCE_UNSPECIFIED')

    def test_documented_override_stays_unknown(self):
        self.monster['creature']['stats']['max_health'] = 200
        self.manifest['entries'].append({'source_index': 1, 'status': 'mapped', 'destination': '/monster/creature/stats/max_health'})
        rows, _ = self.check()
        self.assertEqual(next(r for r in rows if r['source_field'] == 'maxHealth')['status'], 'UNKNOWN_DOCUMENTED_SOURCE_OVERRIDE')
        del self.monster['creature']['stats']['max_health']
        self.assertIn('maxHealth: SOURCE_VALUE_DROPPED', self.check()[1])

    def test_empty_voice_schedule_is_independently_inapplicable(self):
        self.raw.update({'voices.chance': 5, 'voices.interval': 1000})
        self.assertFalse(self.check()[1])
        self.raw['voices[1].text'] = 'Audible'
        self.assertIn('voices.chance: SOURCE_VALUE_DROPPED', self.check()[1])

    def test_zero_corpse_and_literal_empty_collections_cannot_add_native_effects(self):
        self.raw.update({'corpse':0, 'loot':{}, 'attacks':{}, 'voices':{}, 'summon':{}, 'events':{}})
        self.monster['behavior'] = {'attacks':[], 'event_bindings':[]}
        for field in ('corpse','loot','attacks','voices','summon','events'):
            self.manifest['entries'].append({'source_field':field, 'source_file':'example.lua', 'source_index':0, 'status':'approved_omission'})
        rows, errors = self.check();self.assertFalse(errors)
        self.assertTrue(all(next(r for r in rows if r['source_field']==field)['status']=='VERIFIED_APPROVED_NO_EFFECT'
                            for field in ('corpse','loot','attacks','voices','summon','events')))
        self.monster['creature']['corpse_item'] = {'key':'unexpected'}
        self.monster['behavior']['attacks'] = [{'ability':'unexpected'}]
        self.assertIn('corpse: SOURCE_NO_EFFECT_MISMATCH', self.check()[1])
        self.assertIn('attacks: SOURCE_NO_EFFECT_MISMATCH', self.check()[1])
        self.monster['behavior']['event_bindings'] = [{'handler':'unexpected'}]
        self.assertIn('events: SOURCE_NO_EFFECT_MISMATCH', self.check()[1])
        self.monster['loot'] = {'entries':[{'item':'wiki-added'}]}
        self.assertIn('loot: SOURCE_NO_EFFECT_MISMATCH', self.check()[1])
        self.manifest['entries'].append({'source_index':1,'status':'mapped','destination':'/monster/loot/entries/0'})
        rows, errors = self.check()
        self.assertEqual(next(r for r in rows if r['source_field']=='loot')['status'], 'UNKNOWN_DOCUMENTED_SOURCE_OVERRIDE')
        self.assertNotIn('loot: SOURCE_NO_EFFECT_MISMATCH', errors)

    def test_exact_ratio_proof_detects_value_mutation_and_boolean_numerator(self):
        self.raw['defenses.mitigation'] = 2.76
        self.monster['creature']['stats']['mitigation_percent'] = {'numerator': 69, 'denominator': 25}
        rows, errors = self.check()
        self.assertFalse(errors)
        self.assertEqual(next(r for r in rows if r['source_field'] == 'defenses.mitigation')['status'], 'VERIFIED_ACCEPTED_NORMALIZATION')
        self.monster['creature']['stats']['mitigation_percent']['numerator'] = 68
        self.assertIn('defenses.mitigation: SOURCE_VALUE_MISMATCH', self.check()[1])
        self.raw['defenses.mitigation'] = 1
        self.monster['creature']['stats']['mitigation_percent'] = {'numerator': True, 'denominator': 1}
        self.assertIn('defenses.mitigation: SOURCE_VALUE_MISMATCH', self.check()[1])

    def test_bestiary_threshold_enum_and_taxonomy_use_actual_carriers(self):
        self.raw.update({'Bestiary.FirstUnlock': 10, 'Bestiary.SecondUnlock': 100, 'Bestiary.toKill': 250,
                         'Bestiary.Occurrence': 3, 'Bestiary.race': '@BESTY_RACE_MAMMAL'})
        self.monster['creature']['bestiary'] = {'kill_thresholds': [10,100,250], 'occurrence': 'very_rare', 'taxonomy': 'mammal'}
        self.assertFalse(self.check()[1])
        self.monster['creature']['bestiary']['kill_thresholds'][1] = 99
        self.assertIn('Bestiary.SecondUnlock: SOURCE_VALUE_MISMATCH', self.check()[1])
        self.monster['creature']['bestiary']['occurrence'] = 'common'
        self.assertIn('Bestiary.Occurrence: SOURCE_VALUE_MISMATCH', self.check()[1])
        self.assertEqual(values.normalized_expected('Bestiary.race', '@BESTY_RACE_NONE', self.raw), (None, None))
        self.assertEqual(values.normalized_expected('Bestiary.race', '@BESTY_RACE_INVENTED', self.raw), (None, None))

    def test_preserved_stars_cannot_hide_wrong_derived_difficulty(self):
        self.raw['Bestiary.Stars'] = 2
        self.monster['creature']['bestiary'] = {'stars':2, 'difficulty':'easy'}
        self.assertFalse(self.check()[1])
        self.monster['creature']['bestiary']['difficulty'] = 'challenging'
        self.assertIn('Bestiary.Stars: SOURCE_DERIVED_VALUE_MISMATCH', self.check()[1])
        self.manifest['entries'].append({'source_index':1, 'status':'mapped', 'destination':'/monster/creature/bestiary/difficulty'})
        rows,errors=self.check();self.assertFalse(errors)
        proof=next(r for r in rows if r['source_field']=='Bestiary.Stars')['derived_value_proof']
        self.assertEqual(proof['status'], 'UNKNOWN_DOCUMENTED_DERIVATION_OVERRIDE')
        del self.monster['creature']['bestiary']['difficulty']
        self.assertIn('Bestiary.Stars: SOURCE_DERIVED_VALUE_DROPPED', self.check()[1])

    def test_active_light_encoding_cannot_disguise_missing_color(self):
        self.raw.update({'light.level': 3, 'light.color': 215})
        self.monster['presentation'] = {'light': {'level':3, 'color_binding':'canary.appearance:light-color/215'}}
        self.assertFalse(self.check()[1])
        del self.monster['presentation']['light']['color_binding']
        self.assertIn('light.color: SOURCE_VALUE_DROPPED', self.check()[1])
        self.raw['light.level'] = 0; self.monster['presentation']['light']['level'] = 0
        rows, errors = self.check();self.assertFalse(errors)
        self.assertEqual(next(r for r in rows if r['source_field']=='light.color')['status'], 'NOT_APPLICABLE_IN_SOURCE')

    def test_strategy_weight_and_eligibility_false_cannot_silently_drop(self):
        self.raw.update({'strategiesTarget.nearest':70, 'flags.isPreyable':False})
        self.monster['behavior'] = {'targeting': {'strategy_weights': {'nearest':70}}}
        self.monster['creature']['system_eligibility'] = {'prey':False}
        self.assertFalse(self.check()[1])
        self.monster['behavior']['targeting']['strategy_weights']['nearest'] = 69
        self.assertIn('strategiesTarget.nearest: SOURCE_VALUE_MISMATCH', self.check()[1])
        del self.monster['creature']['system_eligibility']['prey']
        self.assertIn('flags.isPreyable: SOURCE_VALUE_DROPPED', self.check()[1])

    def test_mana_default_stays_unknown_and_enabled_mana_cannot_drop(self):
        self.raw['manaCost'] = 0
        del self.raw['flags.summonable']; del self.raw['flags.convinceable']
        self.assertFalse(self.check()[1])
        self.monster['creature']['summoning']['summonable'] = True
        self.assertIn('manaCost: SOURCE_VALUE_DROPPED', self.check()[1])

    def test_register_snapshot_cannot_be_rewritten_after_registration(self):
        rows, error = values.evaluate(SOURCE+'\nmonster.maxHealth=999')
        self.assertIsNone(error)
        self.assertEqual(rows[0]['leaves']['maxHealth'], 100)

    def test_evaluator_retains_enum_false_and_unfinished_registration(self):
        rows, error = values.evaluate(SOURCE.replace('monster.maxHealth=100', 'monster.faction=FACTION_LION\nmonster.maxHealth=100') + '\nMissingFunction()')
        self.assertEqual(rows[0]['leaves']['faction'], '@FACTION_LION')
        self.assertIs(rows[0]['leaves']['flags.attackable'], False)
        self.assertIsNotNone(error)

    def test_bounded_evaluator_cannot_execute_os_or_unbounded_loop(self):
        self.assertIsNotNone(values.evaluate('os.execute("noop")')[1])
        self.assertIn('instruction bound', values.evaluate('while true do end')[1])

    def test_calls_in_comments_and_strings_are_not_proof(self):
        calls = values.registered_calls('-- fake:drop()\nlocal txt="fake:drop()"\nmType:register(monster)')
        self.assertEqual([(r['method'], r['line']) for r in calls], [('register', 3)])

    def test_fresh_paths_and_missing_registered_method_are_detected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); (root/'src').mkdir()
            (root/'register.lua').write_text('registerMonsterType.unregistered=function() end; if mask.flags.newField then mtype:unregistered(mask.flags.newField) end')
            (root/'src/monster_type_functions.cpp').write_text('registerMethod(L,"MonsterType","isAttackable",handler);')
            census = {'sources': {'canary': {'repository': 'opentibiabr/canary', 'revision': values.PINS['canary'],
                      'register_path': 'register.lua', 'mask_paths_with_lines': {}, 'spell_paths_with_lines': {}}}}
            with patch.object(registrar.subprocess, 'check_output', return_value=values.PINS['canary']+'\n'), patch.object(registrar.subprocess, 'run'):
                fresh, evidence = registrar.fresh_inventory(census, {'canary': root})
            self.assertIn('mask.flags.newField', evidence['canary']['new_paths'])
            self.assertFalse(evidence['canary']['registrar_calls'][0]['registered'])
            self.assertIn('canary:mask.flags.newField', registrar.verify_census(fresh)['unclassified'])
            with patch.object(registrar.subprocess, 'check_output', return_value='unbound\n'):
                with self.assertRaises(ValueError):
                    registrar.fresh_inventory(census, {'canary': root})


if __name__ == '__main__':
    unittest.main()
