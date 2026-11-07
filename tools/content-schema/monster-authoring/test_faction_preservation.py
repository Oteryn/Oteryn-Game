"""Faction source-loss regressions: conversion, closed failures, and pinned enum/default evidence."""
import os
import re
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import canary_batch as cb
import crystal_batch
import validate_monster as vm

CANARY, CRYSTAL = os.environ.get('OTERYN_CANARY'), os.environ.get('OTERYN_CRYSTAL')


def convert_source(fields):
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        path = root / cb.MONSTER_DIR / 'probe.lua'
        path.parent.mkdir(parents=True)
        path.write_text('local mType = Game.createMonsterType("Probe")\nlocal monster = {}\n'
                        'monster.race = "fire"\nmonster.health = 100\nmonster.maxHealth = 100\n'
                        + fields + '\nmType:register(monster)\n')
        with patch.object(cb, 'load_effect_constants', return_value=({}, {})):
            converter = cb.Converter(root, {}, {}, {}, {})
        converter.pending_definitions = set()
        return converter.convert('probe')


class FactionConversion(unittest.TestCase):
    def test_relationships_and_preferences_survive_conversion_with_provenance(self):
        _, monster, deps, catalog, manifest, _ = convert_source(
            'monster.faction = FACTION_LION\n'
            'monster.enemyFactions = {FACTION_PLAYER, FACTION_LIONUSURPERS, FACTION_PLAYER}\n'
            'monster.targetPreferPlayer = true\nmonster.targetPreferMaster = false')
        self.assertEqual({'faction': 'FACTION_LION', 'enemy_factions': ['FACTION_LIONUSURPERS', 'FACTION_PLAYER'],
                          'prefer_player': True, 'prefer_master': False}, monster['behavior']['faction_and_preferences'])
        self.assertEqual([], vm.validate(monster, deps, catalog, manifest))
        rows = {row['source_field']: row for row in manifest['entries'] if row['source_field'] in cb.FACTION_FIELDS}
        self.assertEqual(set(cb.FACTION_FIELDS), set(rows))
        for field, row in rows.items():
            self.assertEqual('mapped', row['status'])
            self.assertGreater(row['source_line'], 1)
            self.assertEqual('/monster/behavior/faction_and_preferences/' + cb.FACTION_FIELDS[field], row['destination'])

    def test_preferences_only_get_engine_defaults_and_no_fabricated_enemies(self):
        payload, errors = cb.faction_preferences({'targetPreferMaster': True})
        self.assertEqual({}, errors)
        self.assertEqual({'faction': 'FACTION_DEFAULT', 'enemy_factions': [],
                          'prefer_player': False, 'prefer_master': True}, payload)

    def test_absent_fields_do_not_change_unrelated_creatures(self):
        _, monster, _, _, manifest, _ = convert_source('')
        self.assertNotIn('faction_and_preferences', monster['behavior'])
        self.assertFalse(any(row['source_field'] in cb.FACTION_FIELDS for row in manifest['entries']))

    def test_numeric_unknown_and_malformed_factions_block_manifest(self):
        for fields, expected in (
            ('monster.faction = 2', 'faction'),
            ('monster.faction = FACTION_UNKNOWN', 'faction'),
            ('monster.enemyFactions = {FACTION_PLAYER, 999}', 'enemyFactions'),
            ('monster.enemyFactions = {named = FACTION_PLAYER}', 'enemyFactions'),
            ('monster.targetPreferPlayer = 1', 'targetPreferPlayer')):
            with self.subTest(fields=fields):
                _, monster, deps, catalog, manifest, _ = convert_source(fields)
                self.assertNotIn('faction_and_preferences', monster['behavior'])
                row = next(row for row in manifest['entries'] if row['source_field'] == expected)
                self.assertEqual('unresolved_semantics', row['status'])
                self.assertTrue(vm.validate(monster, deps, catalog, manifest))

    def test_false_masks_and_false_enemies_follow_registrar(self):
        payload, errors = cb.faction_preferences({'faction': False, 'enemyFactions': [False, '@FACTION_PLAYER']})
        self.assertEqual({}, errors)
        self.assertEqual('FACTION_DEFAULT', payload['faction'])
        self.assertEqual(['FACTION_PLAYER'], payload['enemy_factions'])


@unittest.skipUnless(CANARY and CRYSTAL, 'set the pinned Canary and Crystal checkout paths')
class PinnedFactions(unittest.TestCase):
    def test_registered_labels_and_defaults_match_pinned_engine(self):
        canary = Path(CANARY)
        self.assertEqual(cb.REVISION, crystal_batch.git(canary, 'rev-parse', 'HEAD'))
        enum = (canary / 'src/game/game_definitions.hpp').read_text()
        registrations = (canary / 'src/lua/functions/core/game/lua_enums.cpp').read_text()
        defaults = (canary / 'src/creatures/monsters/monsters.hpp').read_text()
        for name in cb.FACTIONS:
            self.assertRegex(enum, r'\b' + name + r'\s*=\s*\d+')
            self.assertIn('registerEnum(L, ' + name + ');', registrations)
        self.assertIn('Faction_t faction = FACTION_DEFAULT;', defaults)
        self.assertIn('bool targetPreferPlayer = false;', defaults)
        self.assertIn('bool targetPreferMaster = false;', defaults)
        registrar = (canary / 'data/scripts/lib/register_monster_type.lua').read_text()
        self.assertIn('for _, enemyFaction in pairs(mask.enemyFactions)', registrar)
        self.assertIn('stdext::vector_set<Faction_t> enemyFactions;', defaults)

    def test_every_pinned_explicit_faction_reaches_the_existing_behavior_schema(self):
        baseline = convert_source('')[1]
        for checkout, prefix, revision in ((Path(CANARY), cb.MONSTER_DIR, cb.REVISION),
                (Path(CRYSTAL), crystal_batch.MONSTER_ROOT, crystal_batch.REVISION)):
            self.assertEqual(revision, crystal_batch.git(checkout, 'rev-parse', 'HEAD'))
            if prefix == cb.MONSTER_DIR:
                objects = cb.load_appearance_objects(checkout / 'data/items/appearances.dat')
                items = cb.load_items_xml(checkout / 'data/items/items.xml')
                names, index = cb.name_index(objects, items)
                converter = cb.Converter(checkout, objects, items, names, index)
            else:
                converter = crystal_batch.converter(Path(CANARY), checkout)
            selected = []
            tracked = crystal_batch.git(checkout, 'ls-tree', '-r', '--name-only', revision, '--', prefix).splitlines()
            for file in tracked:
                if not file.endswith('.lua'):
                    continue
                path = checkout / file
                if not re.search(r'^monster\.(faction|enemyFactions|targetPreferPlayer|targetPreferMaster)\s*=',
                                 path.read_text(), re.M):
                    continue
                _, raw, _ = cb.load_monster(path)
                payload, errors = cb.faction_preferences(raw)
                self.assertEqual({}, errors, file)
                self.assertIsNotNone(payload, file)
                converter.pending_definitions = set()
                _, monster, _, _, manifest, _ = converter.convert(str(path.relative_to(checkout / prefix))[:-4])
                self.assertEqual(payload, monster['behavior']['faction_and_preferences'], file)
                rows = {row['source_field']: row for row in manifest['entries'] if row['source_field'] in cb.FACTION_FIELDS}
                self.assertEqual({key for key in cb.FACTION_FIELDS if key in raw}, set(rows), file)
                self.assertTrue(all(row['status'] == 'mapped' for row in rows.values()), file)
                # Isolate this accepted Behavior section from unrelated source defects.
                self.assertEqual([], vm.structural('monster.schema.json', {
                    **baseline, 'behavior': {**baseline['behavior'],
                                                        'faction_and_preferences': payload}}), file)
                selected.append(file)
            self.assertGreaterEqual(len(selected), 39)


if __name__ == '__main__':
    unittest.main()
