#!/usr/bin/env python3
"""Unit tests for npc_presence_stage.py (NPC-CONTENT-2)."""
from __future__ import annotations

import json
import unittest
from unittest import mock

import npc_presence_stage as stage


def bundle(key, *, speed=None, floor=None, voices=None):
    return {'key': key, 'definition': {'movement': {'floor_change': floor, 'base_speed': speed}},
            'voices': voices}


def voices(interval=5000, chance=10, lines=(('Hello!', False),)):
    return {'interval_ms': interval, 'chance_percent': chance,
            'lines': [{'text': {'text': text}, 'yell': yell} for text, yell in lines]}


def fields(packet, key):
    row = next(row for row in packet['npcs'] if row['key'] == key)
    return {f['field_path'].removeprefix(stage.FIELD): f['value'] for f in row['fields']}


class StageTest(unittest.TestCase):
    def run_stage(self, bindings, sources):
        with mock.patch.object(stage, 'npc_bindings', return_value=bindings):
            return stage.stage(sources)

    def test_single_source_uses_engine_defaults_and_admits_voices(self):
        packet = self.run_stage([('oteryn:npc.a', [('canary', 'a')])],
                                {'canary': {'a': bundle('a', voices=voices())}, 'crystal': {}, 'crystal-summer': {}})
        got = fields(packet, 'oteryn:npc.a')
        self.assertEqual(got['base_speed'], {'type': 'Integer', 'value': 55})
        self.assertEqual(got['floor_change'], {'type': 'Boolean', 'value': False})
        self.assertEqual(got['voice_interval_ms']['value'], 5000)
        self.assertEqual(got['voice_chance_percent']['value'], 10)
        self.assertEqual(json.loads(got['voice_lines']['value']), [{'text': 'Hello!', 'yell': False}])

    def test_differing_voices_are_held_and_say_nothing(self):
        packet = self.run_stage([('oteryn:npc.a', [('canary', 'a'), ('crystal', 'a')])],
                                {'canary': {'a': bundle('a', voices=voices())},
                                 'crystal': {'a': bundle('a', voices=voices(chance=20))}, 'crystal-summer': {}})
        self.assertNotIn('voice_lines', fields(packet, 'oteryn:npc.a'))
        self.assertEqual(packet['held'], [{'key': 'oteryn:npc.a', 'facts': 'voices', 'reason': 'SOURCES_DIFFER'}])

    def test_differing_movement_is_held(self):
        packet = self.run_stage([('oteryn:npc.a', [('canary', 'a'), ('crystal', 'a')])],
                                {'canary': {'a': bundle('a', speed=100)}, 'crystal': {'a': bundle('a')},
                                 'crystal-summer': {}})
        self.assertEqual(packet['npcs'], [])
        self.assertEqual(packet['counts']['held_movement'], 1)

    def test_empty_lines_and_zero_chance_are_no_voice(self):
        silent = voices(chance=0)
        empty = voices(lines=(('', False),))
        packet = self.run_stage([('oteryn:npc.a', [('canary', 'a')]), ('oteryn:npc.b', [('canary', 'b')])],
                                {'canary': {'a': bundle('a', voices=silent), 'b': bundle('b', voices=empty)},
                                 'crystal': {}, 'crystal-summer': {}})
        self.assertEqual(packet['counts']['voiced'], 0)
        self.assertEqual(packet['held'], [])

    def test_undeclared_timing_is_held(self):
        packet = self.run_stage([('oteryn:npc.a', [('canary', 'a')])],
                                {'canary': {'a': bundle('a', voices=voices(interval=0))}, 'crystal': {},
                                 'crystal-summer': {}})
        self.assertEqual(packet['held'], [{'key': 'oteryn:npc.a', 'facts': 'voices', 'reason': 'TIMING_UNDECLARED'}])

    def test_dynamic_voice_text_is_rejected(self):
        dynamic = voices()
        dynamic['lines'][0]['text'] = {'expr': 'player:getName()'}
        with self.assertRaises(stage.StageError):
            stage.voices_of(bundle('a', voices=dynamic))

    def test_unsourced_npc_stages_nothing(self):
        packet = self.run_stage([('oteryn:npc.a', [])], {'canary': {}, 'crystal': {}, 'crystal-summer': {}})
        self.assertEqual((packet['npcs'], packet['counts']['unsourced']), ([], 1))

    def test_committed_packet_is_canonical(self):
        raw = stage.PACKET.read_bytes()
        self.assertEqual(stage.canonical(json.loads(raw)), raw)


if __name__ == '__main__':
    unittest.main()
