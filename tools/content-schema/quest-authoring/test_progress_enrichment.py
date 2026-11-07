"""Regression cases for NPC-written gate tracks and source-qualified aliases."""
import unittest
from collections import Counter
from unittest.mock import patch
from pathlib import Path
from tempfile import TemporaryDirectory
import ots_questlog as questlog


class ProgressEnrichmentTests(unittest.TestCase):
    def setUp(self):
        self.path = 'quest/example/access'
        self.key = 'canary:quest-progress/' + self.path
        self.quest = 'canary:quest/example'
        self.repos = {'canary': '/unused', 'crystalserver': '/unused'}
        self.catalogue = [{'identity': {'key': self.quest}, 'display_name': 'Example', 'missions': []}]
        self.gate = {'identity': {'key': 'canary:door-gate/example'},
                     'quest': {'key': self.quest}, 'condition': {'progress': self.key}}
        self.declarations = {self.key: {'storage_id': 44502, 'path': 'lib/core/storages.lua',
                                      'line': 7, 'repository': 'opentibiabr/canary', 'revision': 'pinned'}}

    def build(self, index=None, progress=None, gates=None):
        with patch.object(questlog, 'TRACK_OWNERS', {}), patch.object(questlog, 'storage_declarations', return_value=self.declarations):
            return questlog.auxiliary_tracks(index or {}, progress or [], self.catalogue,
                                             gates if gates is not None else [self.gate], self.repos)

    def test_npc_only_gate_writer_is_retained(self):
        entry = {'owner': 'npc', 'callback': 'onSay', 'from': [], 'to': 1, 'script': 'npc/example.lua',
                 'sources': {'canary': {'path': 'data-otservbr-global/npc/example.lua', 'line': 23,
                                        'dialogue': {'keywords': ['yes'], 'topics': [1]}}}}
        found = {'count': Counter({'canary': 1}), 'paths': {('canary', self.path)}, 'transitions': {'write': entry}}
        rows = self.build({'questexampleaccess': found})
        self.assertEqual(rows[0]['writes']['canary'], 1)
        self.assertEqual(rows[0]['transitions'][0]['sources']['canary']['line'], 23)
        self.assertEqual(rows[0]['auxiliary_of'], [self.quest])

    def test_declared_gate_with_no_writer_does_not_invent_initial_or_bounds(self):
        row = self.build()[0]
        self.assertEqual(row['writes'], {'canary': 0, 'crystalserver': 0})
        self.assertEqual(row['transitions'], [])
        self.assertEqual(row['source_storage']['storage_id'], 44502)
        self.assertNotIn('initial', row)
        self.assertNotIn('bounds', row)

    def test_other_namespace_keeps_explicit_alias(self):
        source_key = 'crystalserver:quest-progress/' + self.path
        original = {'key': source_key, 'missions': [self.quest + '#mission'], 'start_of': [self.quest],
                    'writes': {'canary': 1, 'crystalserver': 1}, 'transitions': []}
        row = self.build(progress=[original])[0]
        self.assertEqual(row['key'], self.key)
        self.assertEqual(row['alias_of'], source_key)
        self.assertEqual(row['missions'], [])
        self.assertEqual(row['start_of'], [])

    def test_unknown_owner_stays_explicit(self):
        self.gate['quest'] = None
        row = self.build()[0]
        self.assertEqual(row['auxiliary_of'], [])
        self.assertEqual(row['owner_basis'], 'UNKNOWN')
        self.assertIn('UNKNOWN', row['source_checks']['owner'])

    def test_missing_storage_declaration_fails(self):
        self.declarations.clear()
        row = self.build()[0]
        self.assertIn('UNKNOWN', row['source_checks']['storage_declaration'])
        self.assertNotIn('source_storage', row)

    def test_desert_duplicate_joins_by_page_identity(self):
        source = {'identity': {'key': 'canary:quest/the_desert_dungeon_quest'}, 'wiki': {'pageid': 2496}}
        target = {'identity': {'key': 'canary:quest/desert_dungeon_quest'}, 'wiki': {'pageid': 2496}}
        self.assertIs(questlog.reward_script_owner(source, {}, {2496: [target]}), target)

    def test_duplicate_page_candidates_cannot_guess_owner(self):
        source = {'identity': {'key': 'source'}, 'wiki': {'pageid': 2496}}
        self.assertIsNone(questlog.reward_script_owner(source, {}, {2496: [{}, {}]}))

    def test_matching_title_without_page_identity_does_not_join(self):
        source = {'identity': {'key': 'source'}, 'display_name': 'Same title'}
        self.assertIsNone(questlog.reward_script_owner(source, {}, {}))

    def test_alias_does_not_strip_path_separators(self):
        self.key = 'canary:quest-progress/quest/example/ab'
        self.gate['condition']['progress'] = self.key
        original = {'key': 'crystalserver:quest-progress/quest/example/a_b',
                    'missions': [], 'start_of': [], 'auxiliary_of': [self.quest],
                    'writes': {'canary': 0, 'crystalserver': 1}, 'transitions': []}
        with self.assertRaisesRegex(SystemExit, 'no unique declared path'):
            self.build(progress=[original])

    def test_alias_retains_mission_and_start_owners(self):
        source_key = 'crystalserver:quest-progress/' + self.path
        original = {'key': source_key, 'missions': [self.quest + '#mission'], 'start_of': [self.quest],
                    'writes': {'canary': 1, 'crystalserver': 1}, 'transitions': []}
        self.assertEqual(self.build(progress=[original])[0]['auxiliary_of'], [self.quest])

    def test_alias_rejects_multiple_exact_source_paths(self):
        originals = [{'key': namespace + ':quest-progress/' + self.path,
                      'missions': [], 'start_of': [], 'auxiliary_of': [self.quest],
                      'writes': {'canary': 0, 'crystalserver': 1}, 'transitions': []}
                     for namespace in ('crystalserver', 'another_source')]
        with self.assertRaisesRegex(SystemExit, 'no unique declared path'):
            self.build(progress=originals)

    def test_owner_and_storage_gaps_both_survive(self):
        self.gate['quest'] = None
        self.declarations.clear()
        checks = self.build()[0]['source_checks']
        self.assertIn('UNKNOWN', checks['owner'])
        self.assertIn('UNKNOWN', checks['storage_declaration'])

    def test_different_datapack_storage_ids_do_not_overwrite(self):
        with TemporaryDirectory() as tmp:
            for pack, value in [('data-global', 1), ('data-crystal', 2)]:
                path = Path(tmp) / pack / 'lib/core/storages.lua'
                path.parent.mkdir(parents=True)
                path.write_text('Storage = {Quest = {Example = {Door = ' + str(value) + '}}}')
            declarations = questlog.storage_declarations({'crystalserver': tmp})
        key = 'crystalserver:quest-progress/quest/example/door'
        self.assertEqual(declarations[key]['state'], 'CONFLICT')
        self.assertEqual([d['storage_id'] for d in declarations[key]['candidates']], [1, 2])
        self.assertNotIn('source_storage', questlog.storage_evidence(declarations, key, True))
        self.assertIn('CONFLICT', questlog.storage_evidence(declarations, key, True)['source_checks']['storage_declaration'])

    def test_agreeing_datapacks_retain_first_source_declaration(self):
        with TemporaryDirectory() as tmp:
            for pack in ('data-global', 'data-crystal'):
                path = Path(tmp) / pack / 'lib/core/storages.lua'
                path.parent.mkdir(parents=True)
                path.write_text('Storage = {Quest = {Example = {Door = 1}}}')
            declarations = questlog.storage_declarations({'crystalserver': tmp})
        self.assertEqual(declarations['crystalserver:quest-progress/quest/example/door']['path'],
                         'data-global/lib/core/storages.lua')

    def test_curated_exclusive_npc_track_is_included_without_gate(self):
        source = 'npc/the_dream_master.lua'
        entry = {'owner': 'npc', 'callback': 'onSay', 'from': [], 'to': 1, 'script': source,
                 'sources': {'canary': {'path': 'data-otservbr-global/' + source, 'line': 94,
                                        'dialogue': {'keywords': ['yes'], 'topics': [1]}}}}
        found = {'count': Counter({'canary': 1}), 'paths': {('canary', self.path)}, 'transitions': {'write': entry}}
        owners = {'questexampleaccess': {'quest': self.quest, 'npc_source': source}}
        with patch.object(questlog, 'TRACK_OWNERS', owners), patch.object(questlog, 'storage_declarations', return_value=self.declarations):
            rows = questlog.auxiliary_tracks({'questexampleaccess': found}, [], self.catalogue, [], self.repos)
        self.assertEqual(rows[0]['auxiliary_of'], [self.quest])
        self.assertEqual(rows[0]['transitions'][0]['key'], 'npc_1')

    def test_npc_owner_curation_rejects_other_npc_writer(self):
        entry = {'owner': 'npc', 'callback': 'onSay', 'from': [], 'to': 1, 'script': 'npc/unrelated.lua',
                 'sources': {'canary': {'path': 'data-otservbr-global/npc/unrelated.lua', 'line': 94,
                                        'dialogue': {'keywords': ['yes'], 'topics': [1]}}}}
        found = {'count': Counter({'canary': 1}), 'paths': {('canary', self.path)}, 'transitions': {'write': entry}}
        owners = {'questexampleaccess': {'quest': self.quest, 'npc_source': 'npc/the_dream_master.lua'}}
        with patch.object(questlog, 'TRACK_OWNERS', owners), patch.object(questlog, 'storage_declarations', return_value=self.declarations):
            with self.assertRaisesRegex(SystemExit, 'names tracks that need no record'):
                questlog.auxiliary_tracks({'questexampleaccess': found}, [], self.catalogue, [], self.repos)

    def test_shared_npc_gate_cannot_take_namespace_quest_owner(self):
        scripts = ['npc/the_dream_master.lua', 'npc/the_bone_master.lua']
        transitions = {script: {'owner': 'npc', 'callback': 'onSay', 'from': [], 'to': 1, 'script': script,
                               'sources': {'canary': {'path': 'data-otservbr-global/' + script, 'line': 124,
                                                      'dialogue': {'keywords': ['yes'], 'topics': [1]}}}}
                       for script in scripts}
        found = {'count': Counter({'canary': 2}), 'paths': {('canary', self.path)}, 'transitions': transitions}
        owners = {'questexampleaccess': {'shared_npc_gate': True, 'npc_sources': scripts, 'note': 'shared gate'}}
        with patch.object(questlog, 'TRACK_OWNERS', owners), patch.object(questlog, 'storage_declarations', return_value=self.declarations):
            rows = questlog.auxiliary_tracks({'questexampleaccess': found}, [], self.catalogue, [self.gate], self.repos)
        self.assertEqual(rows[0]['auxiliary_of'], [])
        self.assertEqual(rows[0]['owner_basis'], 'UNKNOWN')

    def test_shared_gate_curation_checks_actual_writer_sources(self):
        owners = {'questexampleaccess': {'shared_npc_gate': True,
                                        'npc_sources': ['npc/the_dream_master.lua', 'npc/the_bone_master.lua'],
                                        'note': 'shared gate'}}
        with patch.object(questlog, 'TRACK_OWNERS', owners), patch.object(questlog, 'storage_declarations', return_value=self.declarations):
            with self.assertRaisesRegex(SystemExit, 'shared NPC gate curation is stale'):
                questlog.auxiliary_tracks({}, [], self.catalogue, [self.gate], self.repos)


if __name__ == '__main__':
    unittest.main()
