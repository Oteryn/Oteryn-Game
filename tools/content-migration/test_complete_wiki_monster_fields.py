"""Tests for semantic corrections and guarded source-backed field patches."""
import copy
import importlib.util
import tempfile
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location('wiki_fields', Path(__file__).with_name('complete_wiki_monster_fields.py'))
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def document():
    return {'creature': {'stats': {'max_health': 100, 'initial_health': 100, 'experience': 10, 'speed': 100, 'armor': 4, 'defense': 11, 'mitigation_percent': {'numerator': 2, 'denominator': 1}}, 'resistances': [{'damage_type': 'earth', 'reduction_percent': {'numerator': 20, 'denominator': 1}}], 'immunities': {'damage_types': ['fire'], 'conditions': ['invisible']}, 'summoning': {'is_familiar': False, 'summonable': False, 'convinceable': False}}}


def page(**fields):
    return {'page_title': 'Example', 'url': 'https://tibiawiki.com.br/wiki/Example', 'revision_id': 123, 'content_sha256': 'a' * 64, 'method': 'REMOTE_DESKTOP_CHROME_CDP_PUBLIC_API', 'fields': fields, 'field_lines': {key: i for i, key in enumerate(fields, 1)}}


def by_pointer(patches):
    return {row['pointer']: row for row in patches}


class WikiFieldTests(unittest.TestCase):
    def test_wrong_baseline_digest_fails_before_any_output(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'population-index.json').write_text('{}')
            with self.assertRaisesRegex(ValueError, 'Baseline population index digest mismatch'):
                MODULE.generate(root, root, root / 'out')
            self.assertFalse((root / 'out').exists())

    def test_health_full_spawn_tracks_max_partial_spawn_is_preserved(self):
        d = document()
        p, _, _ = MODULE.patch_actor('example', d, page(hp='250'))
        pointers = by_pointer(p)
        self.assertEqual(pointers['/creature/stats/initial_health']['value'], 250)
        self.assertEqual(pointers['/creature/stats/max_health']['expected_value'], 100)
        d['creature']['stats']['initial_health'] = 30
        p, _, _ = MODULE.patch_actor('example', d, page(hp='250'))
        self.assertNotIn('/creature/stats/initial_health', by_pointer(p))

    def test_armor_not_shield_and_factor_two_speed_not_guessed(self):
        p, deferred, _ = MODULE.patch_actor('example', document(), page(defense='9', speed='200'))
        pointers = by_pointer(p)
        self.assertEqual(pointers['/creature/stats/armor']['value'], 9)
        self.assertNotIn('/creature/stats/defense', pointers)
        self.assertNotIn('/creature/stats/speed', pointers)
        self.assertTrue(any(r['reason'] == 'ENGINE_DISPLAY_FACTOR_TWO_UNIT_UNRESOLVED' for r in deferred))

    def test_received_damage_removes_contradicting_immunity_and_preserves_other_type(self):
        d = document()
        saved = copy.deepcopy(d)
        p, _, _ = MODULE.patch_actor('example', d, page(fireDmgMod='110%', physicalDmgMod='90%'))
        pointers = by_pointer(p)
        reductions = {r['damage_type']: r['reduction_percent'] for r in pointers['/creature/resistances']['value']}
        self.assertEqual(reductions['fire'], {'numerator': -10, 'denominator': 1})
        self.assertEqual(reductions['physical'], {'numerator': 10, 'denominator': 1})
        self.assertEqual(reductions['earth'], {'numerator': 20, 'denominator': 1})
        self.assertEqual(pointers['/creature/immunities/damage_types']['value'], [])
        self.assertEqual(d, saved)

    def test_familiar_owner_mana_does_not_enable_ordinary_summoning(self):
        d = document()
        d['creature']['summoning'].update({'is_familiar': True, 'familiar': {'mana_cost': 1000}})
        p, _, _ = MODULE.patch_actor('example', d, page(summon='3000'))
        self.assertEqual(list(by_pointer(p)), ['/creature/summoning/familiar/mana_cost'])

    def test_variant_and_blank_values_do_not_replace_accepted_estimate(self):
        p, _, resolved = MODULE.patch_actor('example', document(), page(mitigation='3.4'), variant=True, estimated=True)
        self.assertEqual((p, resolved), ([], []))
        p, _, resolved = MODULE.patch_actor('example', document(), page(mitigation='', hp='0', exp='?'), estimated=True)
        self.assertEqual((p, resolved), ([], []))
        p, _, resolved = MODULE.patch_actor('example', document(), page(mitigation='2.00'), estimated=True)
        self.assertEqual(resolved, ['OWNER_ACCEPTED_NON_GLOBAL_ESTIMATE'])
        self.assertEqual(p[0]['value'], p[0]['expected_value'])

    def test_paralysis_preserves_intrinsic_invisibility(self):
        p, _, _ = MODULE.patch_actor('example', document(), page(immunities='Invisibility, Paralysis'))
        self.assertEqual(by_pointer(p)['/creature/immunities/conditions']['value'], ['invisible', 'paralyze'])

    def test_general_creature_class_is_not_a_bestiary_class(self):
        d = document()
        d['creature']['bestiary'] = {'class': 'Human', 'taxonomy': 'human', 'difficulty': 'easy', 'occurrence': 'common', 'stars': 2, 'kill_thresholds': [25, 250, 500], 'charm_points': 15, 'locations': 'Native narrative'}
        p, _, _ = MODULE.patch_actor('example', d, page(creatureclass='Demônios', dificuldade='fácil', ocorrencia='comum', charm='15'), helpers={'difficulty': page()})
        self.assertFalse(any(row['pointer'].startswith('/creature/bestiary') for row in p))

    def test_rare_intermediate_unlocks_are_not_invented(self):
        d = document()
        d['creature']['bestiary'] = {'class': 'Human', 'taxonomy': 'human', 'difficulty': 'easy', 'occurrence': 'very_rare', 'kill_thresholds': [1, 3, 5], 'charm_points': 30}
        p, _, _ = MODULE.patch_actor('example', d, page(dificuldade='fácil', ocorrencia='muito raro', charm='30'), helpers={'difficulty': page()})
        self.assertNotIn('/creature/bestiary/kill_thresholds', by_pointer(p))

    def test_bestiary_unknown_applicability_does_not_create_boss_record(self):
        p, _, _ = MODULE.patch_actor('example', document(), page(isboss='sim', creatureclass='Demônios'))
        self.assertNotIn('/creature/bestiary', by_pointer(p))
        self.assertNotIn('/creature/bosstiary', by_pointer(p))


if __name__ == '__main__':
    unittest.main()
