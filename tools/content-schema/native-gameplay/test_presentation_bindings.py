"""SPELL-PRESENT-0 §3 bindings: the committed files validate, and the checks reject bad bindings."""
import unittest

import build_creature_races as races
import build_presentation_bindings as bindings
import validate_presentation_bindings as validator


def fixture():
    client_ids = {'dynamic_keys': ['canary.appearance:missile/weapontype'], 'records': [
        {'key': 'appearance:effect/poff', 'kind': 'effect', 'client_id': 3,
         'source_member': 'CONST_ME_POFF'},
        {'key': 'canary.appearance:missile/arrow', 'kind': 'missile', 'client_id': 3,
         'source_member': 'CONST_ANI_ARROW'},
        {'key': 'canary.sound:spell_or_rune', 'kind': 'sound', 'client_id': 10,
         'source_member': 'SPELL_OR_RUNE'},
        {'key': 'canary.appearance:effect/none', 'kind': 'effect', 'client_id': 0,
         'source_member': 'CONST_ME_NONE'},
    ]}
    hits = {'player_race': 'blood',
            'physical_by_race': [{'race': race, 'color': 'red'} for race in bindings.RACES],
            'by_damage_type': [{'damage_type': damage, 'color': 'blue'}
                               for damage in validator.DAMAGE_TYPES - {'physical'}]}
    rat = ('oteryn:creature.rat', 'definition-r1')
    creature_races = {'record_count': 1, 'records': [
        {'creature': {'family': 'Creature', 'key': rat[0], 'revision': rat[1]}, 'race': 'blood'}]}
    cues = {'records': [{'alias': 'appearance:effect/poff', 'value': 3}]}
    keys = {record['key'] for record in client_ids['records']} | {'canary.appearance:missile/weapontype'}
    return [client_ids, hits, creature_races, cues, keys,
            {'effect': {3}, 'missile': {3}}, {'red', 'blue'}, {rat}]


class CheckTests(unittest.TestCase):
    def errors(self, change=None):
        arguments = fixture()
        if change:
            change(arguments)
        return validator.check(*arguments)

    def test_fixture_passes(self):
        self.assertEqual(self.errors(), [])

    def test_unbound_and_stale_keys(self):
        def change(a):
            a[4] = a[4] - {'canary.sound:spell_or_rune'} | {'canary.sound:other'}
        self.assertEqual(self.errors(change), ['content key without a client id canary.sound:other',
                                               'client id record no content uses canary.sound:spell_or_rune'])

    def test_dynamic_keys_are_only_the_weapon_missile(self):
        def change(a):
            a[0]['dynamic_keys'].append('canary.sound:other')
            a[4] = a[4] | {'canary.sound:other'}
        self.assertEqual(self.errors(change), [
            'dynamic keys are not exactly the runtime-resolved weapon missile',
            'content key without a client id canary.sound:other'])

    def test_id_missing_from_appearances(self):
        def change(a):
            a[5]['missile'] = {4}
        self.assertEqual(self.errors(change),
                         ['canary.appearance:missile/arrow id 3 is not in appearances.dat'])

    def test_kind_must_match_key(self):
        def change(a):
            a[0]['records'][1].update(kind='sound', client_id=65535)
        self.assertEqual(self.errors(change), [
            'canary.appearance:missile/arrow kind sound is not its key kind missile'])

    def test_null_id_only_for_undefined_member(self):
        def change(a):
            a[0]['records'][0]['client_id'] = None
        self.assertIn('client id missing for appearance:effect/poff', self.errors(change))

    def test_colour_race_damage_and_creature_coverage(self):
        def change(a):
            a[1]['physical_by_race'][0]['color'] = 'white'
            a[1]['physical_by_race'].pop()
            a[1]['by_damage_type'].pop()
            a[2]['records'][0]['race'] = 'slime'
            a[7] = a[7] | {('oteryn:creature.wolf', 'definition-r1')}
        self.assertEqual(self.errors(change), [
            'hit table colour outside PresentationColor white',
            'hit table races are not exactly the source races',
            'hit table damage types are not exactly the Oteryn damage types',
            'creature races do not cover exactly the Creature family',
            'unknown race slime for oteryn:creature.rat'])

    def test_stale_creature_revision(self):
        def change(a):
            a[2]['records'][0]['creature']['revision'] = 'definition-r0'
        self.assertEqual(self.errors(change),
                         ['creature races do not cover exactly the Creature family'])

    def test_spell_cues_disagreement(self):
        def change(a):
            a[3]['records'][0]['value'] = 4
        self.assertEqual(self.errors(change),
                         ['spell-cues.json and client-ids.json disagree on appearance:effect/poff'])


class RaceTests(unittest.TestCase):
    def test_explicit_commented_and_default(self):
        self.assertEqual(races.source_race('monster.race = "undead"\n'), ('undead', 'explicit'))
        self.assertEqual(races.source_race('-- monster.race = "fire"\n'), ('blood', 'source_default'))
        self.assertEqual(races.source_race('--[[\nmonster.race = "fire"\n]]\n'), ('blood', 'source_default'))

    def test_rejects_unknown_or_repeated_race(self):
        with self.assertRaises(ValueError):
            races.source_race('monster.race = "slime"\n')
        with self.assertRaises(ValueError):
            races.source_race('monster.race = "fire"\nmonster.race = "undead"\n')

    def test_primal_variant_reads_its_registering_file(self):
        path = races.SOURCES['canary/monster-file'][2]
        self.assertEqual(path('reptiles/emerald_tortoise.lua#registered-type=Emerald Tortoise (Primal)'),
                         'data-otservbr-global/monster/reptiles/emerald_tortoise.lua')
        self.assertEqual(path('mammals/rat'), 'data-otservbr-global/monster/mammals/rat.lua')


class CommittedTests(unittest.TestCase):
    def test_committed_bindings_validate(self):
        self.assertEqual(validator.main(), 0)

    def test_colours_parse_from_protocol(self):
        self.assertEqual(validator.colors(validator.COLOR_ENUM.read_text()),
                         {'red', 'light_green', 'light_grey', 'orange', 'purple', 'sky_blue', 'yellow',
                          'dark_red', 'light_blue', 'blue', 'pastel_red', 'maya_blue', 'white'})


if __name__ == '__main__':
    unittest.main()
