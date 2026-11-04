"""Source closure and non-executing capture tests for the closed Threatened Dreams data."""
import copy
import json
import tempfile
import unittest
from pathlib import Path

import threatened_dreams_dialogue as authoring


class ThreatenedDreamsAuthoring(unittest.TestCase):
    def test_missing_source_read_flags_keep_unset_and_do_not_change_existing_tracks(self):
        original = json.loads((authoring.ROOT / 'content/quests/missions/quest-state.json').read_bytes())
        before = next(q for q in original['quests'] if q['quest'] == authoring.QUEST)
        result = authoring.state(authoring.ROOT)['quests'][0]
        tracks = {t['key']: t for t in result['tracks']}
        required = [authoring.BASE + 'mission01/poacher_chest'] + [
            authoring.BASE + f'mission04/stone{i}' for i in range(1, 6)]
        for key in required:
            with self.subTest(track=key):
                self.assertIn(key, tracks)
                self.assertEqual((tracks[key]['initial'], tracks[key]['min'], tracks[key]['max']), (-1, -1, 1))
                self.assertEqual(tracks[key]['source_key'], 'crystalserver:' + key.removeprefix('oteryn:'))
        for existing in before['tracks']:
            if existing['key'] in tracks:
                self.assertEqual(tracks[existing['key']], existing, 'existing zero is not migrated')

    def test_conflicting_read_flag_is_rejected_without_overwriting_it(self):
        doc = json.loads((authoring.ROOT / 'content/quests/missions/quest-state.json').read_bytes())
        own = next(q for q in doc['quests'] if q['quest'] == authoring.QUEST)
        key = authoring.BASE + 'mission01/poacher_chest'
        own['tracks'] = [t for t in own['tracks'] if t['key'] != key]
        own['tracks'].append({'key': key, 'quest': authoring.QUEST, 'initial': 0, 'min': 0,
                              'max': 1, 'bounds_basis': 'observed_values',
                              'source_key': 'canary:quest-progress/unknown'})
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            path = root / 'content/quests/missions/quest-state.json'
            path.parent.mkdir(parents=True)
            raw = json.dumps(doc)
            path.write_text(raw)
            with self.assertRaisesRegex(ValueError, 'THREATENED_DREAMS_SOURCE_TRACK_CONFLICT'):
                authoring.state(root)
            self.assertEqual(path.read_text(), raw)

    def test_source_start_and_non_inventory_steps_fit_the_existing_atomic_contract(self):
        rules = {r['key']: r for r in authoring.transitions()}
        start = rules[authoring.TR + 'maelyrra-start']
        self.assertEqual(start['source']['line'], 87)
        self.assertEqual(len(start['effects']), 6)
        self.assertLessEqual(max(len(r['effects']) for r in rules.values()), 8)
        state = {e['track']: e['effect']['value'] for e in start['effects']}
        self.assertEqual(state[authoring.CRYSTAL + 'mission02/enfeebled_count'], 0)
        self.assertEqual(state[authoring.CRYSTAL + 'mission02/frazzlemaws_count'], 0)
        self.assertEqual(state[authoring.CRYSTAL + 'mission02/kroazur_access'], 1)
        self.assertEqual(state[authoring.BASE + 'quest_line'], 2)
        self.assertFalse(any(name in rules for name in (
            authoring.TR + 'maelyrra-kill-report', authoring.TR + 'maelyrra-lights-held',
            authoring.TR + 'maelyrra-blossom-bag', authoring.TR + 'tooth-collection-reward')))

    def test_static_keyword_capture_keeps_source_text_and_dynamic_rules_non_executable(self):
        source = '''keywordHandler:addKeyword({ "deer", "fae" }, StdModule.say, {
            npcHandler = npcHandler, text = { "Line one.", "Line two." }, onlyFocus = true })
keywordHandler:addAliasKeyword({ "stag" })
keywordHandler:addKeyword({ "trade" }, StdModule.say, { npcHandler = npcHandler, text = price .. " coins" })
'''
        captured = authoring.static_keywords(source, authoring.ROOT)
        self.assertEqual(len(captured), 3)
        first = captured[0]
        self.assertEqual(first['line'], 1)
        self.assertEqual(first['literal_messages'], ['Line one.', 'Line two.'])
        self.assertEqual(first['keywords'], ['deer', 'fae'])
        self.assertTrue(first['source_only'])
        self.assertEqual(captured[1]['kind'], 'keyword_alias')
        self.assertIsNone(captured[2]['literal_messages'])
        self.assertIn('price', captured[2]['expression'])

    def test_callback_interceptions_preserve_literal_and_alias_keyword_lists(self):
        source = '''keywordHandler:addKeyword({ "fae", "siblings" }, StdModule.say, {
            npcHandler = npcHandler, text = { "Line one.", "Line two." } })
keywordHandler:addAliasKeyword({ "stag" })
'''
        self.assertEqual(authoring.intercepted_keywords(source, authoring.ROOT), [
            {'line': 1, 'keywords': ['fae', 'siblings']},
            {'line': 3, 'keywords': ['stag']},
        ])

    def test_callback_interceptions_reject_conditional_dynamic_or_effectful_speech(self):
        for source in (
            'keywordHandler:addKeyword({ "name" }, StdModule.say, {npcHandler=npcHandler, text="hi", reset=true})',
            'keywordHandler:addKeyword({ "name" }, StdModule.say, {npcHandler=npcHandler, text="hi"}, function(p) return true end)',
            'keywordHandler:addKeyword({ "name" }, StdModule.say, {npcHandler=npcHandler, text=price .. "hi"})',
            'keywordHandler:addKeyword({ "na.e" }, StdModule.say, {npcHandler=npcHandler, text="hi"})',
            'keywordHandler:addAliasKeyword({ "name" })',
        ):
            with self.subTest(source=source), self.assertRaises(ValueError):
                authoring.intercepted_keywords(source, authoring.ROOT)

    def test_kroazur_exit_has_exact_position_and_aid_without_entry_requirements(self):
        callback = authoring.kroazur_exit()['callbacks'][0]
        self.assertEqual(callback['selector'], {
            'position': {'x': 33591, 'y': 32321, 'floor': -10},
            'unique': None, 'action': 25029, 'appearance': None})
        self.assertEqual(callback['teleport'], {'x': 33619, 'y': 32306, 'floor': -9})
        self.assertEqual(callback['storage'], [])
        self.assertEqual(callback['offerings'], [])
        self.assertFalse(callback['return_on_failure'])
        self.assertEqual(callback['trigger'], 'step_in')
        self.assertTrue(any('movement_kroazur_room.lua:3-35' in s for s in callback['sources']))

    def test_poacher_discovery_uses_all_three_exact_fields_and_preserves_stage_guard(self):
        callbacks = authoring.poacher_discovery()['callbacks']
        self.assertEqual(len(callbacks), 3)
        self.assertEqual([c['selector']['position'] for c in callbacks], [
            {'x': x, 'y': 31811, 'floor': -7} for x in (32949, 32950, 32951)])
        for callback in callbacks:
            self.assertEqual(callback['trigger'], 'step_in')
            self.assertEqual(callback['storage'][0], {
                'track': authoring.TRACKS['animals'], 'minimum': 6, 'maximum': 6, 'set': 7})
            self.assertEqual(callback['storage'][1], {
                'track': authoring.TRACKS['animals_log'], 'minimum': 0, 'maximum': 16, 'set': 7})
            self.assertFalse(callback['return_on_failure'])
            self.assertIsNone(callback['teleport'])
            self.assertIsNone(callback['item_on_target'])
            self.assertTrue(any('movement_poacher_notes.lua:8-17' in s for s in callback['sources']))


if __name__ == '__main__':
    unittest.main()
