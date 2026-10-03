"""NPC writes supplement existing mission families; never invent shared ownership."""
import copy
import unittest
from collections import Counter
from unittest.mock import patch
import ots_questlog as q


class NpcProgressRetentionTests(unittest.TestCase):
    def setUp(self):
        self.path = 'quest/u1_0/example/access'
        self.key = 'canary:quest-progress/' + self.path
        self.quest = 'canary:quest/example'
        self.catalogue = [{'identity': {'key': self.quest}, 'display_name': 'Example',
                           'missions': [{'progress': 'canary:quest-progress/quest/u1_0/example/mission'}]}]
        self.entry = {'owner': 'npc', 'callback': None, 'from': None, 'to': 2,
                      'script': 'npc/mentor.lua', 'sources': {'canary': {
                          'path': 'data-otservbr-global/npc/mentor.lua', 'line': 12,
                          'registrations': [], 'dialogue': {'keywords': [], 'topics': []}}}}
        self.found = {'count': Counter({'canary': 1}), 'paths': {('canary', self.path)},
                      'transitions': {'one': self.entry}}
        self.declarations = {self.key: {'storage_id': 100, 'path': 'data-otservbr-global/lib/core/storages.lua',
                                       'line': 8, 'expression': 'Storage.Quest.U1_0.Example.Access'}}

    def build(self, owners=None):
        with patch.object(q, 'TRACK_OWNERS', owners or {}), patch.object(q, 'storage_declarations', return_value=self.declarations):
            return q.auxiliary_tracks({q.norm(self.path): self.found}, [], self.catalogue, [],
                                      {'canary': '/unused', 'crystalserver': '/unused'})

    def test_single_npc_auxiliary_write_retained_without_gate_or_curator(self):
        row = self.build()[0]
        self.assertEqual(row['auxiliary_of'], [self.quest])
        self.assertEqual(row['owner_basis'], 'mission track prefix')
        self.assertEqual(row['writes'], {'canary': 1, 'crystalserver': 0})
        self.assertEqual(row['transitions'][0]['write']['to'], 2)
        self.assertEqual(row['transitions'][0]['sources']['canary']['line'], 12)
        self.assertEqual(row['source_storage']['storage_id'], 100)
        self.assertNotIn('initial', row)
        self.assertNotIn('bounds', row)

    def test_computed_value_stays_computed(self):
        self.entry.pop('to'); self.entry['computed'] = 'expression'
        row = self.build()[0]
        self.assertEqual(row['transitions'][0]['write']['computed'], 'expression')
        self.assertNotIn('to', row['transitions'][0]['write'])

    def test_ambiguous_mission_family_not_admitted(self):
        other = copy.deepcopy(self.catalogue[0]); other['identity']['key'] = 'canary:quest/other'
        self.catalogue.append(other)
        self.assertEqual(self.build(), [])

    def test_broad_version_prefix_not_admitted(self):
        self.catalogue[0]['missions'][0]['progress'] = 'canary:quest-progress/quest/u1_0/other/mission'
        self.assertEqual(self.build(), [])

    def test_multiple_npc_callers_not_inferred_as_one_quest(self):
        other = copy.deepcopy(self.entry); other['script'] = 'npc/second.lua'
        self.found['transitions']['two'] = other
        self.assertEqual(self.build(), [])

    def test_colliding_normalized_paths_not_admitted(self):
        self.found['paths'].add(('crystalserver', 'quest/u1_0/ex_am_ple/access'))
        self.assertEqual(self.build(), [])

    def test_missing_declaration_keeps_explicit_unknown(self):
        self.declarations.clear()
        row = self.build()[0]
        self.assertIn('UNKNOWN', row['source_checks']['storage_declaration'])
        self.assertNotIn('source_storage', row)

    def test_curated_npc_writer_guard_has_precedence_over_prefix(self):
        owners = {q.norm(self.path): {'quest': self.quest, 'npc_source': 'npc/expected.lua'}}
        row = self.build(owners)[0]
        self.assertEqual(row['owner_basis'], 'UNKNOWN')
        self.assertEqual(row['auxiliary_of'], [])
        self.assertIn('writer set changed', row['source_checks']['owner'])


if __name__ == '__main__':
    unittest.main()
