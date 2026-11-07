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

    def test_curated_exact_fields_only_and_idempotence(self):
        old = copy.deepcopy(self.quests)
        rows, checks = self.apply()
        self.assertEqual(self.quests, old)
        keys = {c['quest'] for c in self.curations}
        self.assertEqual([q for q in rows if q['identity']['key'] not in keys],
                         [q for q in old if q['identity']['key'] not in keys])
        self.assertEqual({c['min_level'] for c in checks}, {0, 2, 8, 80, 100, 275})
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
        self.assertEqual(len(holds), 7)
        self.assertEqual(list(holds.values()), [1] * 7)
        notes = [c for c in checks if c.get('scope_note')]
        self.assertEqual(len(notes), 4)
        for c in notes:
            entry = next(e for e in self.curations if e['quest'] == c['quest'])
            self.assertEqual(c['scope_note'], entry['scope_note'])
            old = next(q for q in self.quests if q['identity']['key'] == c['quest'])
            self.assertEqual(c['min_level'], old['requirements']['min_level'])
        for q in rows:
            if q['identity']['key'] in holds:
                check = next(c for c in checks if c['quest'] == q['identity']['key'])
                self.assertEqual(q['requirements_unparsed']['min_level'], 'note' if check.get('scope_note') else 'range')
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
        for raw in ['?', '0?', '0 (100 For The Firewalker Boots Part)', '77*(last mission)', '10 / 12', '20 - 2']:
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
        scoped = copy.deepcopy(checks)
        note = next(c for c in scoped if c.get('scope_note'))
        note.pop('scope_note'); note['coverage_gap'] = None
        with self.assertRaises(ValueError): requirement_holds(scoped, keys)


    def test_explicit_none_is_unrestricted_with_source_proof_only(self):
        rows, checks = self.apply()
        absent = [c for c in checks if c['level_raw'] == 'None']
        self.assertEqual(len(absent), 3)
        for check in absent:
            self.assertEqual(check['source_literal'], 'None')
            self.assertEqual(check['interpretation'], 'explicit_no_level_requirement')
            q = next(q for q in rows if q['identity']['key'] == check['quest'])
            self.assertEqual(q['requirements']['min_level'], 0)
            self.assertEqual(q['requirements_from_wiki']['lvl'], 'None')
            self.assertNotIn('min_level', q['requirements_unparsed'])
            definition = quest_tree_authoring.build_records([q], [])[0]['definition']
            self.assertEqual(definition['readiness'], 'waiting_data')
            self.assertEqual(definition['native_lowering']['state'], 'WAITING_IMPLEMENTATION')
        self.assertEqual(parse_level('None'), (None, 'none'))

    def test_none_cannot_replace_blank_note_or_different_source(self):
        entry = next(e for e in self.curations if e['level_raw'] == 'None')
        for raw, note in [(None, None), ('', None), ('?', None), ('None', 'optional mission only')]:
            facts = copy.deepcopy(self.facts)
            page = next(p for p in facts['fresh_wiki']['pages'] if p['provider'] == 'tibia_fandom' and p['pageid'] == entry['wiki']['pageid'])
            page['source_fields'].update(level=raw, level_note=note)
            with self.subTest(raw=raw, note=note), self.assertRaises(ValueError): self.apply(facts=facts)
        entries = copy.deepcopy(self.curations)
        next(e for e in entries if e['level_raw'] == 'None')['wiki']['provider'] = 'tibiawiki_br'
        with self.assertRaises(ValueError): self.apply(curations=entries)
        entries = copy.deepcopy(self.curations); facts = copy.deepcopy(self.facts)
        next(e for e in entries if e['quest'] == entry['quest'])['scope_note'] = 'mission only'
        next(p for p in facts['fresh_wiki']['pages'] if p['provider'] == 'tibia_fandom' and p['pageid'] == entry['wiki']['pageid'])['source_fields']['level_note'] = 'mission only'
        with self.assertRaises(ValueError): self.apply(facts=facts, curations=entries)

    def test_none_witness_annotation_cannot_be_erased(self):
        rows, checks = self.apply(); keys = {q['identity']['key'] for q in rows}
        index = next(i for i, c in enumerate(checks) if c['level_raw'] == 'None')
        for field in ['source_literal', 'interpretation']:
            changed = copy.deepcopy(checks); changed[index].pop(field)
            with self.subTest(field=field), self.assertRaises(ValueError): requirement_holds(changed, keys)


if __name__ == '__main__':
    unittest.main()
