import copy
import hashlib
import unittest

from quest_catalogue_authoring import build


class CatalogueChecks(unittest.TestCase):
    def setUp(self):
        self.coverage = {'wiki': {'retrieved_at': '2026-09-27T10:10:10Z'}, 'servers': {}, 'quests': [
            {'title': 'Small Quest', 'pageid': 1, 'revid': 2, 'rev_timestamp': '2026-09-20T00:00:00Z',
             'in_quest_log': False, 'premium': 'no', 'lvl': '0', 'implemented': '7.0'}]}
        self.quest = {'display_name': 'Family', 'identity': {'key': 'source:quest/family', 'revision': 'pin'},
                      'kind': 'storyline', 'shown_in_quest_log': True, 'missions': [{'key': 'small'}]}
        self.links = {'links': [{'wiki_title': 'Small Quest', 'wiki_pageid': 1, 'wiki_revid': 2,
                                'target_key': 'source:quest/family', 'mission_keys': ['small'],
                                'scope': 'partial_named_missions', 'basis': 'Named mission', 'evidence': {'path': 'fixture'}}]}
        self.facts = {'source_backed_authoring': []}
        self.provenance = [{'path': 'fixture-' + str(i), 'sha256': 'a' * 64} for i in range(4)]

    def run_build(self, **changes):
        values = {'coverage': self.coverage, 'quests': [self.quest], 'readiness': {'quests': []},
                  'links': self.links, 'facts': self.facts, 'provenance': self.provenance}
        values.update(changes)
        return build(**values)

    def test_family_is_partial_and_not_runtime_ready(self):
        row = self.run_build()['quests'][0]
        self.assertEqual(row['coverage_state'], 'represented_by_family')
        self.assertEqual(row['family_representation']['scope'], 'partial_named_missions')
        self.assertEqual(row['global_rewards']['classification'], 'UNKNOWN')

    def test_stale_revision_target_and_mission_rejected(self):
        for field, value in [('wiki_revid', 3), ('target_key', 'missing'), ('mission_keys', ['missing'])]:
            links = copy.deepcopy(self.links)
            links['links'][0][field] = value
            with self.assertRaises(ValueError):
                self.run_build(links=links)

    def test_false_complete_rejected(self):
        self.links['links'][0]['scope'] = 'complete'
        with self.assertRaises(ValueError):
            self.run_build()

    def test_ambiguous_family_link_rejected(self):
        self.links['links'].append(copy.deepcopy(self.links['links'][0]))
        with self.assertRaises(ValueError):
            self.run_build()

    def test_missing_provenance_rejected(self):
        with self.assertRaises(ValueError):
            self.run_build(provenance=[])
        del self.coverage['quests'][0]['revid']
        with self.assertRaises(ValueError):
            self.run_build()

    def test_duplicate_source_and_authored_keys_rejected(self):
        coverage = copy.deepcopy(self.coverage)
        coverage['quests'].append(copy.deepcopy(coverage['quests'][0]))
        with self.assertRaises(ValueError):
            self.run_build(coverage=coverage)
        with self.assertRaises(ValueError):
            self.run_build(quests=[self.quest, copy.deepcopy(self.quest)])

    def test_log_conflict_keeps_both_values_and_duplicate_title(self):
        first = {**self.quest, 'display_name': 'Small Quest'}
        second = {**first, 'identity': {'key': 'source:quest/second', 'revision': 'pin'}}
        row = self.run_build(quests=[first, second], links={'links': []})['quests'][0]
        self.assertEqual(row['coverage_state'], 'conflict')
        self.assertFalse(row['source_facts']['in_quest_log'])
        self.assertTrue(row['authored_candidates'][0]['ots_shown_in_quest_log'])
        self.assertEqual({c['kind'] for c in row['source_checks']}, {'duplicate_title', 'quest_log_conflict'})

    def fresh_fixture(self):
        row = {'provider': 'tibiawiki_br', 'pageid': 3, 'revid': 4, 'content_sha256': 'a' * 64,
               'revision_timestamp': '2026-09-20T00:00:00Z', 'url': 'https://example.test/?oldid=4',
               'target_cut': '2026-10-01', 'access_method': 'remote_desktop_browser',
               'source_fields': {'listing_complete': False, 'level': '2', 'premium': 'sim'}}
        marker = {k: row[k] for k in ('provider', 'pageid', 'revid', 'content_sha256')}
        fingerprint = 'tibiawiki_br:3:4:' + 'a' * 64 + '\n'
        fresh = {'pages': [row], 'revision_manifest': [marker], 'unavailable_pages': [],
                 'snapshot_pages_digest': hashlib.sha256(fingerprint.encode()).hexdigest(),
                 'source_target_cuts': {'tibiawiki_br': '2026-10-01'},
                 'title_links': [{'wiki_title': 'Small Quest', 'provider': 'tibiawiki_br', 'pageid': 3,
                                  'revid': 4, 'role': 'crosscheck', 'mapping_basis': 'Exact title'}]}
        return {'source_backed_authoring': [], 'fresh_wiki': fresh}

    def test_fresh_missing_revision_and_stale_content_rejected(self):
        for missing in [True, False]:
            facts = self.fresh_fixture()
            row = facts['fresh_wiki']['pages'][0]
            if missing:
                del row['revid']
            else:
                row['content_sha256'] = 'b' * 64
            with self.assertRaises(ValueError):
                self.run_build(facts=facts)

    def test_fresh_source_difference_preserves_cached_values(self):
        row = self.run_build(facts=self.fresh_fixture())['quests'][0]
        self.assertEqual(row['source_facts']['lvl'], '0')
        self.assertEqual(row['source_facts']['premium'], 'no')
        self.assertEqual(row['fresh_sources'][0]['source_fields']['level'], '2')
        self.assertEqual({c['field'] for c in row['source_checks']}, {'level', 'premium'})
        self.assertTrue(all(c['kind'] == 'structured_source_difference' for c in row['source_checks']))

    def test_fresh_conflict_marks_matching_authored_quest(self):
        quest = {**self.quest, 'display_name': 'Small Quest', 'shown_in_quest_log': False}
        row = self.run_build(quests=[quest], links={'links': []}, facts=self.fresh_fixture())['quests'][0]
        self.assertEqual(row['coverage_state'], 'conflict')
        self.assertEqual(row['source_facts']['lvl'], '0')
        self.assertEqual(row['fresh_sources'][0]['source_fields']['level'], '2')
        self.assertTrue(all(c['kind'] == 'structured_source_difference' for c in row['source_checks']))

    def test_fresh_listing_cannot_claim_complete(self):
        facts = self.fresh_fixture()
        facts['fresh_wiki']['pages'][0]['source_fields']['listing_complete'] = True
        with self.assertRaises(ValueError):
            self.run_build(facts=facts)


if __name__ == '__main__':
    unittest.main()
