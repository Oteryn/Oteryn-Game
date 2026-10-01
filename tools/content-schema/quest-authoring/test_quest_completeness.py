"""Regressions for truthful source coverage, joins and unresolved references."""
import copy
import json
import unittest
from pathlib import Path

import ots_gap_triage as triage
import ots_readiness as readiness
from refresh_quest_source_checks import refresh
from validate_quest_content import (BLOCKED, gate_source_checks, interaction_source_checks,
                                    validate_gates, validate_interactions)


TRACK = 'oteryn:quest-progress/other'


def quest(name):
    return {'identity': {'key': 'oteryn:quest/' + name, 'revision': 'r1'},
            'kind': 'script_only', 'claims': []}


def interaction(script='named/script', rules=None):
    return {'identity': {'key': 'oteryn:interaction/' + script, 'revision': 'r1'},
            'source': {'edge': 'USE', 'callback': 'onUse', 'target_registrations': []},
            'rules': rules or [], 'anchors': [], 'unresolved': []}


def predicate():
    return {'quest_stage': {'progress': TRACK, 'op': '==', 'value': 1}, 'negate': False}


def manifest(inter):
    return {'counts': {}, 'undeclared_progress_tracks': [], 'entries': [
        {'destination': inter['identity']['key'], 'status': 'mapped', 'sources': [{'source': 'canary'}]}]}


class CompletenessTests(unittest.TestCase):
    def test_named_directory_survives_cross_quest_outfit_reward(self):
        qs = [quest('feaster_of_souls_quest'), quest('poltergeist_outfits_quest')]
        progress = [{'key': TRACK, 'start_of': [], 'auxiliary_of': [qs[1]['identity']['key']]}]
        boss = interaction('feaster_of_souls/pale_worm', [
            {'owner': 'Quest', 'request': 'set_progress', 'progress': TRACK, 'to': 1}])
        portal = interaction('feaster_of_souls/portal')
        joined = readiness.join_interactions(qs, progress, [boss, portal])
        self.assertEqual(joined[portal['identity']['key']], {qs[0]['identity']['key']})
        self.assertEqual(joined[boss['identity']['key']], {q['identity']['key'] for q in qs})

    def test_ambiguous_directory_inference_stays_unlinked(self):
        qs = [quest('one'), quest('two')]
        progress = [{'key': TRACK, 'start_of': [qs[0]['identity']['key']], 'auxiliary_of': []},
                    {'key': TRACK + '_two', 'start_of': [qs[1]['identity']['key']], 'auxiliary_of': []}]
        scripts = [interaction('unmatched/a', [{'owner': 'Quest', 'progress': TRACK}]),
                   interaction('unmatched/b', [{'owner': 'Quest', 'progress': TRACK + '_two'}]),
                   interaction('unmatched/no_tracks')]
        self.assertEqual(readiness.join_interactions(qs, progress, scripts)[scripts[-1]['identity']['key']], set())

    def test_blocked_child_is_data_gap_even_with_accepted_owner(self):
        for owner in ('WorldObject', 'Movement'):
            rule = {'owner': owner, 'status': 'blocked', 'reason': BLOCKED[owner],
                    'source_line' if owner == 'WorldObject' else 'to_source_line': 7}
            inter = interaction(rules=[rule])
            self.assertEqual(readiness.interaction_facts(inter)[2], 1)
            self.assertTrue(validate_interactions({'interactions': [inter]}, manifest(inter),
                                                 {'quests': []}, {'progress': []}))
            m = manifest(inter)
            m['entries'][0]['status'] = 'unresolved_semantics'
            self.assertEqual(validate_interactions({'interactions': [inter]}, m,
                                                  {'quests': []}, {'progress': []}), [])

    def test_missing_nested_read_requires_exact_inventory_and_unresolved_status(self):
        inter = interaction(rules=[{'branch': [{'when': {'any': [predicate(), {'actor_is_player': True, 'negate': False}]}, 'then': []}]}])
        doc, progress = {'interactions': [inter]}, {'progress': []}
        m = manifest(inter)
        self.assertTrue(validate_interactions(doc, m, {'quests': []}, progress))
        m['source_checks'] = interaction_source_checks(doc, progress)
        self.assertTrue(validate_interactions(doc, m, {'quests': []}, progress))
        m['entries'][0]['status'] = 'unresolved_semantics'
        self.assertEqual(validate_interactions(doc, m, {'quests': []}, progress), [])
        self.assertEqual(readiness.interaction_facts(inter, set())[2], 1)
        m['source_checks'].append(copy.deepcopy(m['source_checks'][0]))
        self.assertTrue(validate_interactions(doc, m, {'quests': []}, progress))

    def test_resolved_read_rejects_stale_diagnostic(self):
        inter = interaction(rules=[{'branch': [{'when': predicate(), 'then': []}]}])
        doc, m = {'interactions': [inter]}, manifest(inter)
        m['source_checks'] = interaction_source_checks(doc, {'progress': []})
        self.assertTrue(validate_interactions(doc, m, {'quests': []}, {'progress': [{'key': TRACK}]}))
        m['source_checks'] = []
        self.assertEqual(validate_interactions(doc, m, {'quests': []}, {'progress': [{'key': TRACK}]}), [])

    def test_gate_missing_quest_and_progress_are_explicit_and_never_mapped(self):
        gate = {'identity': {'key': 'oteryn:door-gate/progress/named', 'revision': 'r1'}, 'label': None,
                'quest': {'family': 'Quest', 'key': 'oteryn:quest/named', 'revision': 'r1'},
                'quest_link_basis': 'storage_key', 'condition': {'kind': 'quest_progress', 'progress': TRACK, 'claim': None},
                'state': 'per_character_pass', 'placements': [{'position': {'x': 1, 'y': 2, 'z': 7}, 'appearance': None}]}
        gates, quests, progress = {'gates': [gate]}, {'quests': []}, {'progress': []}
        m = {'counts': {}, 'entries': [{'destination': gate['identity']['key'], 'status': 'mapped'}]}
        self.assertTrue(validate_gates(gates, {'claims': []}, m, quests, progress))
        m['source_checks'] = gate_source_checks(gates, quests, progress)
        self.assertEqual(len(m['source_checks']), 2)
        self.assertTrue(validate_gates(gates, {'claims': []}, m, quests, progress))
        m['entries'][0]['status'] = 'unresolved_semantics'
        self.assertEqual(validate_gates(gates, {'claims': []}, m, quests, progress), [])
        progress['progress'].append({'key': TRACK})
        self.assertTrue(validate_gates(gates, {'claims': []}, m, quests, progress))

    def test_refresh_is_idempotent_and_does_not_invent_content(self):
        inter = interaction(rules=[{'branch': [{'when': predicate(), 'then': []}]}])
        gates, gm, quests, tracks, ints, im = ({'gates': []}, {'counts': {}, 'entries': []},
            {'quests': []}, {'progress': []}, {'interactions': [inter]}, manifest(inter))
        originals = copy.deepcopy((gates, quests, tracks, ints))
        refreshed = refresh(gates, gm, quests, tracks, ints, im)
        self.assertEqual(refreshed, refresh(gates, refreshed[0], quests, tracks, ints, refreshed[1]))
        self.assertEqual(originals, (gates, quests, tracks, ints))
        self.assertEqual(refreshed[1]['entries'][0]['status'], 'unresolved_semantics')

    def test_refresh_preserves_unrelated_gate_source_hold(self):
        gm = {'counts': {}, 'entries': [{'destination': 'oteryn:door-gate/key/1234',
                                        'status': 'unresolved_semantics'}]}
        updated, _ = refresh({'gates': []}, gm, {'quests': []}, {'progress': []},
                             {'interactions': []}, {'counts': {}, 'entries': []})
        self.assertEqual(updated['entries'][0]['status'], 'unresolved_semantics')

    def test_committed_regressions_and_shared_triage_join(self):
        samples = Path(__file__).parent / 'samples'
        load = lambda path, key: json.loads((samples / path).read_text())[key]
        qs, ts, ins = (load('questlog/quests.json', 'quests'), load('questlog/progress.json', 'progress'),
                       load('interactions/interactions.json', 'interactions'))
        joined = readiness.join_interactions(qs, ts, ins)
        feaster = [i for i in ins if '/feaster_of_souls/' in i['identity']['key']]
        self.assertEqual(len(feaster), 10)
        self.assertTrue(all('canary:quest/feaster_of_souls_quest' in joined[i['identity']['key']] for i in feaster))
        triage_join, _, _ = triage.build_quest_join()
        self.assertEqual(triage_join, {key: owners for key, owners in joined.items() if owners})


if __name__ == '__main__':
    unittest.main()
