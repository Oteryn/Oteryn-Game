"""QUEST-LOWER-1: source progress lowered into QuestState tracks and transitions."""
import copy
import json
import unittest
from pathlib import Path

import quest_state_lowering as tool

ROOT = Path(__file__).resolve().parents[3]


def write(key, to=None, increment=None, computed=None, source_from=None, npc=None):
    value = {'callback': None, 'from': source_from, 'key': key, 'owner': 'npc' if npc else 'action', 'servers': ['canary']}
    if to is not None:
        value['to'] = to
    if increment is not None:
        value['increment'] = increment
    if computed is not None:
        value['computed'] = computed
    if npc:
        value['requested_by'] = {'keywords': ['yes'], 'npc': npc, 'topics': [1]}
    return {'key': key, 'script': 'npc/x.lua', 'source_occurrences': [], 'sources': {}, 'write': value}


def progress(key, transitions, missions=()):
    return {'key': key, 'missions': list(missions), 'read_by_gates': [], 'start_of': [], 'transitions': transitions, 'writes': {}}


def definition(name, tracks, missions=()):
    return {'identity': {'key': 'oteryn:quest.' + name, 'revision': 'quest-r1'},
            'source_data': {'quest': {'identity': {'key': 'canary:quest/' + name}, 'missions': list(missions)}, 'progress': tracks}}


def mission(track, start, end):
    return {'progress': track, 'start_value': start, 'end_value': end}


STAGE = 'canary:quest-progress/quest/x/stage'
COUNT = 'crystalserver:quest-progress/quest/x/count'


def lower(records):
    owned = tool.owners(records)
    values = tool.mission_values(records)
    quests = [tool.lower_quest(d, owned, values) for d in records]
    tool.validate(quests)
    return quests


class QuestStateLoweringTests(unittest.TestCase):
    def test_keys_bounds_effects_and_bindings(self):
        quest = lower([definition('x', [
            progress(STAGE, [write('npc_1', to=1, npc='canary:npc/x'),
                             write('npc_2', to=-1, source_from={'op': '==', 'value': 3, 'exact': False}),
                             write('action_1', computed='expression')], missions=['canary:quest/x#m']),
            progress(COUNT, [write('action_1', increment=1, source_from={'op': '<', 'value': 4, 'exact': True})]),
        ], missions=[mission(STAGE, 1, 3)])])[0]
        stage, count = (next(t for t in quest['tracks'] if t['source_key'] == key) for key in (STAGE, COUNT))
        self.assertEqual(stage['key'], 'oteryn:quest-progress/quest/x/stage')
        self.assertEqual(count['key'], 'oteryn:quest-progress/crystalserver/quest/x/count')
        self.assertEqual((stage['initial'], stage['min'], stage['max']), (-1, -1, tool.STORAGE_WIDTH_MAX))
        self.assertEqual((count['initial'], count['min'], count['max']), (0, 0, tool.STORAGE_WIDTH_MAX))
        transitions = {t['source']['key'] + '@' + t['effects'][0]['track']: t for t in quest['transitions']}
        npc = transitions['npc_1@' + stage['key']]
        self.assertEqual(npc['key'], 'oteryn:quest-transition/quest/x/stage/npc_1')
        self.assertEqual(npc['effects'][0]['from'], {'op': 'ANY'})
        self.assertEqual(npc['effects'][0]['effect'], {'kind': 'SET', 'value': 1})
        self.assertEqual(npc['requested_by'], {'npc': 'canary:npc/x', 'keywords': ['yes'], 'topics': [1]})
        inexact = transitions['npc_2@' + stage['key']]['effects'][0]
        self.assertEqual((inexact['from'], inexact['from_exact']), ({'op': 'EQ', 'value': 3}, False))
        self.assertEqual(transitions['action_1@' + stage['key']]['effects'][0]['effect'], {'kind': 'COMPUTED'})
        self.assertEqual(transitions['action_1@' + count['key']]['effects'][0]['effect'], {'kind': 'ADD', 'value': 1})
        self.assertFalse(any(t['completes'] for t in quest['transitions']))

    def test_set_only_tracks_take_observed_bounds_and_timestamps_are_set_now(self):
        quest = lower([definition('x', [progress(STAGE, [write('npc_1', to=5)]),
                                        progress(COUNT, [write('action_1', computed='timestamp')])])])[0]
        stage, count = sorted(quest['tracks'], key=lambda t: t['source_key'] != STAGE)
        self.assertEqual((stage['initial'], stage['min'], stage['max']), (0, 0, 5))
        self.assertEqual((count['max'], count['bounds_basis']), (tool.I64_MAX, 'set_now_unbounded'))
        self.assertEqual(quest['completion'], 'NOT_LOWERED_NO_MISSIONS')

    def test_completes_only_on_a_single_mission_track_set_to_its_end(self):
        quest = lower([definition('x', [progress(STAGE, [write('npc_1', to=1), write('npc_2', to=4)])],
                                  missions=[mission(STAGE, 1, 2), mission(STAGE, 2, 4)])])[0]
        self.assertEqual(quest['completion'], 'LOWERED')
        self.assertEqual([t['source']['key'] for t in quest['transitions'] if t['completes']], ['npc_2'])
        quest = lower([definition('x', [progress(STAGE, [write('npc_2', to=4)]), progress(COUNT, [])],
                                  missions=[mission(STAGE, 1, 4), mission(COUNT, 0, 1)])])[0]
        self.assertEqual(quest['completion'], 'NOT_LOWERED_MULTI_TRACK')
        self.assertFalse(any(t['completes'] for t in quest['transitions']))

    def test_a_shared_track_belongs_to_its_first_missions_quest(self):
        shared = progress(STAGE, [write('npc_1', to=1)], missions=['canary:quest/b#m', 'canary:quest/a#n'])
        a, b = lower([definition('a', [copy.deepcopy(shared)]), definition('b', [copy.deepcopy(shared)])])
        self.assertEqual((a['tracks'], [t['quest'] for t in b['tracks']]), ([], ['oteryn:quest.b']))
        with self.assertRaises(tool.LoweringError):
            tool.owners([definition('a', [shared]), definition('b', [progress(STAGE, [])])])

    def test_unknown_source_shapes_fail_closed(self):
        for bad in (write('npc_1', to=1, source_from={'op': '!', 'value': 1, 'exact': True}),
                    write('npc_1', computed='random'), write('npc_1')):
            with self.assertRaises(tool.LoweringError):
                lower([definition('x', [progress(STAGE, [bad])])])
        with self.assertRaises(tool.LoweringError):
            lower([definition('x', [progress('kv:quest-progress/x', [])])])
        with self.assertRaises(tool.LoweringError):
            lower([definition('x', [progress('canary:quest-progress/' + 'a' * 120, [])])])

    def test_committed_lowering_is_current_and_has_no_source_store_key(self):
        files = tool.expected(ROOT)
        for relative, text in files.items():
            self.assertEqual((ROOT / relative).read_text(encoding='utf-8'), text, relative)
        payload = json.loads(files[tool.OUTPUT])
        for quest in payload['quests']:
            keys = [quest['quest'], *(t['key'] for t in quest['tracks']), *(t['key'] for t in quest['transitions']),
                    *(e['track'] for t in quest['transitions'] for e in t['effects'])]
            self.assertTrue(all(tool.valid_key(key) for key in keys), quest['quest'])


if __name__ == '__main__':
    unittest.main()
