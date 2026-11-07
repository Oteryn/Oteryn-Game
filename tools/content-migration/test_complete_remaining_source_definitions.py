import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import complete_remaining_source_definitions as batch


class SourceCompletionTests(unittest.TestCase):
    def test_restored_source_packet_retains_placement_debt_and_66_original_omissions(self):
        receipt = Path('/workspace/monster-final-output/sources-final/completion.json')
        if not receipt.exists():
            self.skipTest('Generated source packet is not present in this checkout')
        report = json.loads(receipt.read_text())
        restored = {r['monster'] for r in report['actors'] if r['restored_cores']}
        for row in report['index_monsters']:
            if row['monster'] in restored:
                self.assertIn('ENCOUNTER_PLACEMENT_PENDING', row['completion_flags'])
        self.assertEqual(sum(len(r['omitted_behavior']) for r in report['actors']), 66)

    def test_fire_typo_correction_retains_original_call_and_explicit_non_global_flag(self):
        original = {'combats': {0: {'param_calls': [['COMBAT_PARAM_TYPE', 'COMBAT_FIREDAMAGE'],
                                                   ['COMBAT_PARAM_SHOOT_EFFECT', 'CONST_ANI_FIRE']]}}}
        instance = object.__new__(batch.CorrectedSpellScripts)
        with patch.object(batch.spell_scripts.SpellScripts, '_evaluate', return_value=copy.deepcopy(original)):
            fixed = instance._evaluate('targetfirering')
        self.assertEqual(fixed['combats'][0]['param_calls'][1][0], 'COMBAT_PARAM_DISTANCEEFFECT')
        self.assertEqual(fixed['oteryn_source_correction']['original_combat_calls'], original['combats'])
        self.assertFalse(fixed['oteryn_source_correction']['global_confirmed'])

    def test_inventory_path_cannot_escape_source_root(self):
        for value in ('../rat', '/rat', 'foo\\bar', 'rat.lua'):
            with self.subTest(value=value), self.assertRaises(ValueError):
                batch.checked_relative({'source_file': value}, 'data-global/monster')
        self.assertEqual(batch.checked_relative({'source_file': 'data-global/monster/plants/rat'},
                                                'data-global/monster'), 'plants/rat')

    def test_omission_retains_exact_original_debt_and_removes_false_destination(self):
        original = {'source_field': 'attacks[2]', 'status': 'unresolved_semantics',
                    'resolution': 'custom callback unsupported', 'destination': '/absent'}
        manifest = {'entries': [copy.deepcopy(original)]}
        debt = batch.qualify(manifest)
        self.assertEqual(debt, [original])
        self.assertEqual(manifest['entries'][0]['status'], 'approved_omission')
        self.assertNotIn('destination', manifest['entries'][0])
        self.assertIn(original['resolution'], manifest['entries'][0]['resolution'])

    def test_numeric_dependency_and_partial_table_uncertainty_still_block(self):
        for status, field in [('unresolved_dependency', 'loot[1]'),
                              ('unsupported_source_field', 'health'),
                              ('partial_text', 'description'),
                              ('unresolved_semantics', 'top-level script before mType:register')]:
            with self.subTest(status=status), self.assertRaises(ValueError):
                batch.qualify({'entries': [{'source_field': field, 'status': status}]})

    def test_corpse_omission_preserves_loot_and_exact_original_item_proof(self):
        corpse = {'family': 'Item', 'key': 'canary:item/48397'}
        m = {'creature': {'corpse_item': corpse}, 'loot': {'entries': [{'item': {'key': 'canary:item/3031'}}]}}
        d = {'items': [{'identity': {'key': 'canary:item/48397'}, 'temporal': {'decay_target': {'key': 'canary:item/48396'}}},
                       {'identity': {'key': 'canary:item/48396'}}, {'identity': {'key': 'canary:item/3031'}}]}
        c = {'definitions': [{'key': 'canary:item/48397'}, {'key': 'canary:item/48396'}, {'key': 'canary:item/3031'}]}
        manifest = {'entries': [{'destination': '/monster/creature/corpse_item', 'status': 'mapped', 'source_field': 'corpse'}]}
        loot = copy.deepcopy(m['loot'])
        receipt = batch.omit_unadmitted_corpse(m, d, c, manifest, 'rootthing_nutshell')
        self.assertEqual(m['loot'], loot)
        self.assertNotIn('corpse_item', m['creature'])
        self.assertEqual(receipt['corpse_reference'], corpse)
        self.assertEqual(len(receipt['corpse_items']), 2)
        self.assertEqual(d['items'], [{'identity': {'key': 'canary:item/3031'}}])
        self.assertEqual(manifest['entries'][0]['status'], 'approved_omission')

    def test_recovery_reads_later_stats_without_guessed_interval_or_callback_execution(self):
        source = '''local mType = Game.createMonsterType("Goshnar's Cruelty")
local monster = {}
monster.attacks = {
{ name = "melee", interval = 2000 },
{ name = "cruelty transform elemental", interval = SoulWarQuest.goshnarsCrueltyWaveInterval * 1000, chance = 50 },
}
monster.defenses = { armor = 160, defense = 160, mitigation = 5.40 }
monster.health = 300000
monster.loot = {{id=3031, chance=55000}}
mType.onThink = function() error("must never execute callback") end
mType:register(monster)
'''
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'goshnars_cruelty.lua'
            path.write_text(source)
            name, raw, callbacks = batch.recover_table(path)
            self.assertEqual(path.read_text(), source)
        self.assertEqual(name, "Goshnar's Cruelty")
        self.assertEqual(raw['health'], 300000)
        self.assertEqual(raw['defenses']['mitigation'], 5.4)
        self.assertEqual(len(raw['attacks']), 1)
        self.assertEqual(raw['loot'][0]['chance'], 55000)
        self.assertEqual(callbacks['onThink'], 'function')

    def test_zone_query_is_removed_only_for_evaluation_not_replaced_with_fake_zone(self):
        source = '''local mType = Game.createMonsterType("Goshnar's Malice")
local monster = { health = 300000, defenses = { mitigation = 5.4 } }
local zone = Zone.getByName("boss.goshnar's-malice")
local zonePositions = zone:getPositions()
mType.onThink = function() return zonePositions end
mType:register(monster)
'''
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'goshnars_malice.lua'
            path.write_text(source)
            name, raw, callbacks = batch.recover_table(path)
            self.assertEqual(path.read_text(), source)
            path.write_text(source.replace('zone:getPositions()', 'zone:getTiles()'))
            with self.assertRaises(ValueError):
                batch.recover_table(path)
        self.assertEqual(raw['health'], 300000)
        self.assertIn('onThink', callbacks)


if __name__ == '__main__':
    unittest.main()
