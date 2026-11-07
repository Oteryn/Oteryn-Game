"""Preserve executed source data without guessing missing quest configuration."""
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import canary_batch as cb
import validate_monster as vm


class PartialSource(unittest.TestCase):
    def source(self, text, errors=None):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'source.lua'
            path.write_text(text, encoding='utf-8')
            return cb.load_monster(path, errors)

    def test_strict_reads_still_reject_failure_before_registration(self):
        with self.assertRaises(Exception):
            self.source('local mType = Game.createMonsterType("Example")\n'
                        'local monster = {health=123}\nZone.getByName("missing")\n')

    def test_only_executed_values_are_preserved_and_failure_is_explicit(self):
        errors = []
        name, monster, callbacks = self.source(
            'local mType = Game.createMonsterType("Example")\n'
            'local monster = {health=123, maxHealth=456, flags={attackable=true}}\n'
            'mType.onSpawn = function() end\n'
            'monster.attacks = {{interval=QuestConfig.missing * 1000}}\n'
            'monster.experience = 999\n'
            'mType.onThink = function() end\n'
            'mType:register(monster)\n', errors)
        self.assertEqual('Example', name)
        self.assertEqual({'health': 123, 'maxHealth': 456, 'flags': {'attackable': True}}, monster)
        self.assertEqual({'onSpawn': 'function'}, callbacks)
        self.assertEqual(1, len(errors))
        self.assertTrue(errors[0].startswith('PRE_REGISTER: '))
        self.assertIn('missing', errors[0])

    def test_registered_table_takes_priority_over_unrelated_error_local(self):
        errors = []
        name, monster, _ = self.source(
            'local mType = Game.createMonsterType("Example")\n'
            'local monster = {health=123}\nmType:register(monster)\n'
            'local function fail() local monster={health=999}; error("late") end\nfail()\n', errors)
        self.assertEqual('Example', name)
        self.assertEqual({'health': 123}, monster)
        self.assertEqual(1, len(errors))
        self.assertFalse(errors[0].startswith('PRE_REGISTER: '))

    def test_nested_unrelated_table_cannot_replace_the_source_monster(self):
        errors = []
        _, raw, _ = self.source(
            'local mType = Game.createMonsterType("Example")\n'
            'local monster = {health=123}\n'
            'local function fail() local monster={health=999}; error("early") end\nfail()\n', errors)
        self.assertEqual({'health': 123}, raw)
        self.assertTrue(errors[0].startswith('PRE_REGISTER: '))

    def test_helper_or_failed_unrelated_local_does_not_become_monster(self):
        with self.assertRaises(cb.NonRegisteringSourceError):
            self.source('GrandMasterOberonConfig = {AmountLife=3}\n')
        with self.assertRaises(Exception):
            self.source('local monster = {health=999}\nerror("helper failure")\n', [])
        with self.assertRaises(Exception):
            self.source('Game.createMonsterType("Example")\n'
                        'local function fail() local monster={health=999}; error("helper") end\nfail()\n', [])
        with self.assertRaises(cb.NonRegisteringSourceError):
            self.source('Game.createMonsterType("Unregistered")\nlocal monster={health=123}\n', [])

    def test_normal_registration_retains_callback_and_complete_table(self):
        errors = []
        name, raw, callbacks = self.source(
            'local mType = Game.createMonsterType("Example")\n'
            'local monster = {health=123, maxHealth=123}\n'
            'mType.onThink = function() end\nmType:register(monster)\n', errors)
        self.assertEqual('Example', name)
        self.assertEqual({'health': 123, 'maxHealth': 123}, raw)
        self.assertEqual({'onThink': 'function'}, callbacks)
        self.assertEqual([], errors)

    def test_partial_bundle_cannot_pass_manifest_readiness(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / cb.MONSTER_DIR / 'example.lua'
            path.parent.mkdir(parents=True)
            path.write_text('local mType=Game.createMonsterType("Example")\n'
                            'local monster={health=123,maxHealth=123,outfit={lookType=42},race="none"}\n'
                            'Zone.getByName("missing")\nmType:register(monster)\n', encoding='utf-8')
            with patch.object(cb, 'load_effect_constants', return_value=({}, {})):
                converter = cb.Converter(root, {}, {}, {}, {})
            converter.pending_definitions = set()
            _, monster, deps, catalog, manifest, _ = converter.convert('example')
            errors = vm.validate(monster, deps, catalog, manifest)
            self.assertTrue(any('unresolved_semantics' in error for error in errors))
            entry = next(row for row in manifest['entries']
                         if row['source_field'] == 'top-level script before mType:register')
            self.assertEqual('unresolved_semantics', entry['status'])
            self.assertIn('must not be admitted', entry['resolution'])
            self.assertEqual(123, monster['creature']['stats']['max_health'])
            path.write_text('local mType=Game.createMonsterType("Example")\nlocal monster={experience=5}\n'
                            'Zone.getByName("missing")\nmType:register(monster)\n', encoding='utf-8')
            with self.assertRaisesRegex(ValueError, 'required facts were not evaluated: health, maxHealth, outfit'):
                converter.convert('example')

    @unittest.skipUnless(os.environ.get('OTERYN_CANARY'), 'set OTERYN_CANARY to the pinned checkout')
    def test_all_five_pinned_soul_war_failures_retain_raw_stats_without_admission(self):
        base = Path(os.environ['OTERYN_CANARY']) / cb.MONSTER_DIR / 'quests/soul_war'
        names = ["goshnar's_megalomania_blue", "goshnar's_megalomania_green", "goshnar's_megalomania_purple",
                 'goshnars_cruelty', 'goshnars_malice']
        for name in names:
            with self.subTest(name=name):
                errors = []
                _, raw, _ = cb.load_monster(base / (name + '.lua'), errors)
                self.assertGreater(raw['health'], 0)
                self.assertGreater(raw['maxHealth'], 0)
                self.assertEqual(name in ("goshnar's_megalomania_blue", 'goshnars_malice'), 'defenses' in raw)
                self.assertTrue(errors[0].startswith('PRE_REGISTER: '))


if __name__ == '__main__':
    unittest.main()
