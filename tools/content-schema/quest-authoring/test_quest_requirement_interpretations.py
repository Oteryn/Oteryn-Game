"""Exact pinned minimum interpretation keeps interval/source/native boundaries."""
import copy
import json
import unittest
from pathlib import Path

import ots_readiness
import quest_tree_authoring
from ots_chests import parse_level
from quest_requirement_interpretations import interpret_requirements, level_bounds, requirement_holds

HERE = Path(__file__).parent


class RequirementInterpretationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.quests = json.loads((HERE / 'samples/questlog/quests.json').read_text())['quests']
        cls.facts = json.loads((HERE / 'wiki_quest_facts.json').read_text())
        cls.curations = json.loads((HERE / 'quest_requirement_curations.json').read_text())['entries']

    def apply(self, quests=None, facts=None, curations=None):
        return interpret_requirements(quests or self.quests, facts or self.facts, curations or self.curations)

    def test_four_exact_fields_only_and_idempotence(self):
        old = copy.deepcopy(self.quests)
        rows, checks = self.apply()
        self.assertEqual(self.quests, old)
        keys = {c['quest'] for c in self.curations}
        self.assertEqual([q for q in rows if q['identity']['key'] not in keys],
                         [q for q in old if q['identity']['key'] not in keys])
        self.assertEqual({c['min_level'] for c in checks}, {0, 2, 8})
        self.assertEqual(self.apply(quests=rows), (rows, checks))
        for q in rows:
            if q['identity']['key'] in keys:
                source = next(x for x in old if x['identity'] == q['identity'])
                for field in set(q) - {'requirements', 'requirements_unparsed'}:
                    self.assertEqual(q[field], source[field])
                self.assertEqual(q['requirements']['premium'], source['requirements']['premium'])

    def test_interval_holds_consumed_by_real_readiness(self):
        rows, checks = self.apply()
        keys = {q['identity']['key'] for q in rows}
        holds = ots_readiness.quest_coverage_holds({'source_checks': {'quest_requirement_interpretations': checks}}, keys)
        self.assertEqual(len(holds), 3)
        self.assertEqual(list(holds.values()), [1, 1, 1])
        for q in rows:
            if q['identity']['key'] in holds:
                self.assertEqual(q['requirements_unparsed']['min_level'], 'range')
                reports = {q['identity']['key']: {'data_gaps': {'unresolved_items': 1}}}
                definition = quest_tree_authoring.build_records([q], [], reports)[0]['definition']
                self.assertEqual(definition['readiness'], 'waiting_data')
                self.assertIn({'code': 'reported_source_gap', 'field': 'unresolved_items', 'count': 1}, definition['missing_data'])
                self.assertEqual(definition['native_lowering']['state'], 'WAITING_IMPLEMENTATION')

    def test_unqualified_plus_no_recommendation_borrow(self):
        rows, checks = self.apply()
        q = next(q for q in rows if q['identity']['key'].endswith('/their_masters_voice'))
        self.assertEqual(q['requirements']['min_level'], 8)
        self.assertNotIn('min_level', q['requirements_unparsed'])
        self.assertNotIn(q['identity']['key'], requirement_holds(checks, {r['identity']['key'] for r in rows}))

    def test_stale_or_forged_pins_rejected(self):
        for field, value in [('revid', 1), ('content_sha256', '0' * 64), ('pageid', 1), ('title', 'Other Quest')]:
            entries = copy.deepcopy(self.curations); entries[0]['wiki'][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError): self.apply(curations=entries)

    def test_raw_scope_or_note_change_rejected(self):
        for field, value in [('level', '100'), ('level_note', 'for last mission only')]:
            facts = copy.deepcopy(self.facts)
            page = next(p for p in facts['fresh_wiki']['pages'] if p['provider'] == 'tibia_fandom' and p['pageid'] == self.curations[0]['wiki']['pageid'])
            page['source_fields'][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError): self.apply(facts=facts)

    def test_curated_quest_removed_duplicate_or_conflicting_minimum_rejected(self):
        key = self.curations[0]['quest']
        with self.assertRaises(ValueError): self.apply(quests=[q for q in self.quests if q['identity']['key'] != key])
        with self.assertRaises(ValueError): self.apply(curations=self.curations + [self.curations[0]])
        rows = copy.deepcopy(self.quests)
        next(q for q in rows if q['identity']['key'] == key)['requirements']['min_level'] = 999
        with self.assertRaises(ValueError): self.apply(quests=rows)

    def test_no_generic_family_footnote_none_or_uncertain_default(self):
        for raw in ['?', 'None', '0?', '0 (100 For The Firewalker Boots Part)', '77*(last mission)', '10 / 12', '20 - 2']:
            with self.subTest(raw=raw), self.assertRaises(ValueError): level_bounds(raw)
        self.assertEqual(parse_level('8+'), (None, 'range'))
        self.assertEqual(parse_level('2 - 20'), (None, 'range'))

    def test_forged_missing_upper_or_coverage_hold_rejected(self):
        rows, checks = self.apply(); keys = {q['identity']['key'] for q in rows}
        index = next(i for i, c in enumerate(checks) if c['max_level'] is not None)
        for field, value in [('max_level', None), ('coverage_gap', None), ('min_level', False)]:
            changed = copy.deepcopy(checks); changed[index][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError): requirement_holds(changed, keys)
        with self.assertRaises(ValueError): requirement_holds(checks + [checks[0]], keys)
        with self.assertRaises(ValueError): requirement_holds(checks, set())


if __name__ == '__main__':
    unittest.main()
