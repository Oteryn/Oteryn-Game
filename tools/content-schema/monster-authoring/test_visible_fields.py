"""Regression cases for source preservation and honest infobox completeness states."""
import copy
import json
import tempfile
import unittest
from pathlib import Path

import verify_visible_fields as vf


def creature():
    return {
        'identity': {'key': 'canary:creature/demon'}, 'display_name': 'Demon',
        'stats': {'max_health': 8200, 'experience': 6000, 'armor': 44, 'speed': 128,
                  'mitigation_percent': {'numerator': 69, 'denominator': 25}},
        'summoning': {'summonable': False, 'convinceable': False},
        'bestiary': {'charm_points': 50, 'difficulty': 'hard', 'occurrence': 'common', 'stars': 4},
    }


class VisibleFieldsTest(unittest.TestCase):
    def test_committed_demon_profile_matches_user_infobox(self):
        repo = Path(__file__).resolve().parents[3]
        record = next(record for record, _ in vf.profile_records(repo)
                      if record['definition']['identity']['key'] == 'oteryn:creature.demon')
        actual = vf.native_creature(record)
        for field, path in vf.FIELDS.items():
            self.assertEqual(vf.value_at(actual, path), vf.value_at(creature(), path), field)

    def test_every_infobox_field_has_a_formal_schema_destination(self):
        schema = Path(__file__).with_name('monster.schema.json')
        self.assertTrue(all(row['status'] == 'PRESENT' for row in vf.schema_fields(schema).values()))
        definitions = json.loads(schema.read_text())
        del definitions['$defs']['bestiary']['properties']['charm_points']
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'schema.json'
            path.write_text(json.dumps(definitions))
            self.assertEqual(vf.schema_fields(path)['charm_points']['status'], 'SCHEMA_OMISSION')

    def test_demon_complete_and_disabled_actions_have_no_mana_cost(self):
        row = vf.inspect_creature(creature(), creature())
        self.assertEqual(row['problems'], [])
        self.assertEqual(row['fields']['mitigation_percent']['value'], {'numerator': 69, 'denominator': 25})
        self.assertEqual(row['fields']['mana_cost']['status'], 'NOT_APPLICABLE')
        self.assertEqual(sum(e['status'] == 'PRESENT' for e in row['fields'].values()), 11)

    def test_missing_bestiary_is_unknown_without_source_applicability(self):
        c = creature()
        del c['bestiary']
        self.assertEqual(vf.inspect_creature(c)['fields']['charm_points']['status'], 'UNKNOWN')
        self.assertEqual(vf.inspect_creature(c, c)['fields']['charm_points']['status'], 'NOT_APPLICABLE_IN_SOURCE')

    def test_missing_mitigation_is_unspecified_not_zero(self):
        c = creature()
        del c['stats']['mitigation_percent']
        self.assertEqual(vf.inspect_creature(c, c)['fields']['mitigation_percent']['status'], 'SOURCE_UNSPECIFIED')
        self.assertNotIn('value', vf.inspect_creature(c, c)['fields']['mitigation_percent'])

    def test_native_omission_is_a_failure_when_source_has_value(self):
        c = creature()
        del c['stats']['mitigation_percent']
        del c['bestiary']
        row = vf.inspect_creature(c, creature())
        self.assertEqual(len(row['problems']), 5)
        self.assertEqual(row['fields']['charm_points']['status'], 'DATA_OMISSION')

    def test_summon_cost_required_and_false_is_not_unknown(self):
        c = creature()
        c['summoning']['summonable'] = True
        row = vf.inspect_creature(c)
        self.assertEqual(row['fields']['mana_cost']['status'], 'DATA_OMISSION')
        self.assertEqual(row['fields']['convinceable'], {'status': 'PRESENT', 'value': False})
        c['summoning']['mana_cost'] = 0
        self.assertEqual(vf.inspect_creature(c)['fields']['mana_cost']['status'], 'PRESENT')

    def test_invalid_ratios_and_bool_stats_are_not_accepted(self):
        for ratio in ({'numerator': 138, 'denominator': 50}, {'numerator': 1, 'denominator': 0},
                      {'numerator': -1, 'denominator': 1}, {'numerator': 101, 'denominator': 1}):
            self.assertFalse(vf.valid('mitigation_percent', ratio))
        self.assertFalse(vf.valid('experience', True))
        self.assertTrue(vf.valid('speed', 0))
        self.assertTrue(vf.valid('mitigation_percent', {'numerator': 0, 'denominator': 1}))

    def test_changed_stat_is_reported_against_bundle(self):
        c = creature()
        c['stats']['armor'] = 40
        row = vf.inspect_creature(c, creature())
        self.assertEqual(row['fields']['armor']['status'], 'SOURCE_MISMATCH')
        self.assertEqual(row['fields']['armor']['expected'], 44)

    def test_counts_keep_extra_unadmitted_bundles_visible(self):
        c = creature()
        record = {'definition': {'identity': {'key': 'oteryn:creature.demon'}},
                  'authoring': {'profile': {'health': 8200, 'experience': 6000, 'armor': 44, 'speed': 128,
                      'mitigation': c['stats']['mitigation_percent'], 'bestiary': c['bestiary'],
                      'details': {'display_name': 'Demon', 'summoning': c['summoning']}}}}
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            target = repo / 'content/creatures/definitions'
            target.mkdir(parents=True)
            (target / 'creatures-00000-00499.json').write_text(json.dumps({'records': [record]}))
            result = vf.inventory(repo, {'demon': c, 'other': copy.deepcopy(c)})
        self.assertEqual(result['problem_count'], 0)
        self.assertEqual(result['admitted_count'], 1)
        self.assertEqual(result['matched_bundle_count'], 1)
        self.assertEqual(result['unadmitted_bundle_count'], 1)

    def test_stage_profiles_use_the_same_native_field_mapping(self):
        c = creature()
        data = {'kind': 'Creature', 'profile': {'health': 8200, 'experience': 6000,
                'armor': 44, 'speed': 128, 'mitigation': c['stats']['mitigation_percent'],
                'bestiary': c['bestiary'], 'details': {'display_name': 'Demon', 'summoning': c['summoning']}}}
        with tempfile.TemporaryDirectory() as directory:
            stage = Path(directory) / 'stage.json'
            stage.write_text(json.dumps({'authoring_profiles': [
                {'target': {'family': 'Creature', 'key': 'oteryn:creature.demon'}, 'data': data}]}))
            result = vf.inventory(Path(directory), {'demon': c}, stage)
        self.assertEqual(result['admitted_count'], 1)
        self.assertEqual(result['problem_count'], 0)


if __name__ == '__main__':
    unittest.main()
