"""Tests for the D17 source-incomplete record of tools/content-migration/npc_dialogue_stage.py.

Usage: python -m unittest test_npc_dialogue_stage.py
"""
import collections
import hashlib
import re
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'content-migration'))

import npc_dialogue_stage as stage  # noqa: E402


def textref(text):
    return {'sha256': hashlib.sha256(text.encode('utf-8')).hexdigest(), 'length': len(text), 'placeholders': [],
            'links': sorted(set(link.lower() for link in re.findall(r'\{([^{}]+)\}', text))), 'text': text}


def say(keywords, text, children=()):
    return {'kind': 'say', 'gate': 'NONE', 'effect': 'NONE', 'flags': {}, 'keywords': list(keywords),
            'text': textref(text), 'children': list(children)}


def dialogue(*nodes):
    return stage.build_dialogue({'dialogue': {'keywords': list(nodes)}}, collections.Counter())


def entries(*nodes):
    return (dialogue(*nodes) or {}).get('source_incomplete')


STOPPED = ('I can guide you. You must confront its most devoted servant, the majordomo. A {traitor} of the highest '
           'order towards my cause.')


HESITATE = 'I hesitate to ask. Would you consider {helping} me?'


def marrow_chain():
    return [say(['unpleasant'], 'It must be {stopped}.'), say(['stopped'], STOPPED),
            say(['hesitate'], HESITATE), say(['name'], 'Doctor Marrow.')]


def cipfried_tibia():
    return say(['tibia'], "That's where we are: its {citizens}, its {merchants} and its {monsters}.")


class SourceIncompleteTests(unittest.TestCase):
    def test_doctor_marrow_stopped_keeps_its_link_and_records_both_entries(self):
        result = dialogue(*marrow_chain())
        stopped = next(node for node in result['keywords'] if node['key'] == 'stopped')
        self.assertEqual(stopped['links'], ['traitor'])
        self.assertEqual(result['source_incomplete'],
                         [{'node': 'hesitate', 'link': 'helping', 'reason': 'NO_HANDLER'},
                          {'node': 'stopped', 'link': 'traitor', 'reason': 'NO_HANDLER'}])
        declaration = stage.build_declaration('doctor_marrow', result)
        self.assertEqual(declaration['source_incomplete'], result['source_incomplete'])

    def test_every_emitted_node_keeps_its_links(self):
        result = dialogue(*marrow_chain())
        self.assertEqual({node['key']: node.get('links') for node in result['keywords']},
                         {'unpleasant': ['stopped'], 'stopped': ['traitor'], 'hesitate': ['helping'], 'name': None})

    def test_a_link_with_a_handler_adds_no_entry(self):
        nodes = marrow_chain() + [say(['traitor'], 'A traitor indeed.'), say(['helping'], 'Helping.')]
        self.assertIsNone(entries(*nodes))
        self.assertNotIn('source_incomplete', stage.build_declaration('doctor_marrow', dialogue(*nodes)))

    def test_a_link_answered_only_by_a_lua_callback_still_records_an_entry(self):
        # The engine never runs a MsgContains callback: it is no KeywordNode handler (D17(e)), so it is
        # absent from the bundle's keyword tree and the link stays unanswered.
        bundle = {'dialogue': {'keywords': [say(['hesitate'], HESITATE)]},
                  'unresolved': [{'path': 'dialogue.keywords[1]', 'reason': 'LUA_CALLBACK', 'detail': 'helping'}]}
        result = stage.build_dialogue(bundle, collections.Counter())
        self.assertEqual(result['source_incomplete'],
                         [{'node': 'hesitate', 'link': 'helping', 'reason': 'NO_HANDLER'}])

    def test_cipfried_inflected_links_are_answered_by_substring(self):
        nodes = [cipfried_tibia(), say(['citizen'], 'Citizens.'), say(['merchant'], 'Merchants.'),
                 say(['monster'], 'Monsters.')]
        self.assertIsNone(entries(*nodes))

    def test_equality_only_match_fails_the_cipfried_case(self):
        nodes = [cipfried_tibia(), say(['citizen'], 'Citizens.'), say(['merchant'], 'Merchants.'),
                 say(['monster'], 'Monsters.')]
        equality = lambda keywords, label: all(keyword == label for keyword in keywords)
        original = stage.handler_matches
        stage.handler_matches = equality
        try:
            self.assertEqual({e['link'] for e in entries(*nodes)}, {'citizens', 'merchants', 'monsters'})
        finally:
            stage.handler_matches = original
        self.assertIsNone(entries(*nodes))

    def test_every_keyword_of_a_node_must_match(self):
        self.assertEqual([e['link'] for e in entries(say(['x'], 'See {red apple}.'), say(['red', 'pear'], 'No.'))],
                         ['red apple'])
        self.assertIsNone(entries(say(['x'], 'See {red apple}.'), say(['red', 'apple'], 'Yes.')))

    def test_matching_is_case_insensitive(self):
        self.assertIsNone(entries(say(['a'], 'Make {CREATION}!'), say(['Creation'], 'Marvels.')))

    def test_a_link_answered_only_by_an_unemitted_node_adds_no_entry(self):
        gated = say(['traitor'], 'Hidden.')
        gated['gate'] = 'QUEST'  # not emitted, still a source handler
        result = dialogue(*(marrow_chain() + [gated, say(['helping'], 'Helping.')]))
        self.assertNotIn('traitor', [node['key'] for node in result['keywords']])
        self.assertNotIn('source_incomplete', result)

    def test_a_handler_among_the_children_answers_the_link(self):
        parent = say(['stopped'], STOPPED, children=[say(['traitor'], 'Yes.')])
        self.assertIsNone(entries(parent))

    def test_an_enclosing_level_answers_the_link(self):
        child = say(['traitor'], 'Yes.')
        inner = say(['b'], 'Then {traitor}.')
        self.assertIsNone(entries(say(['a'], 'Go {b}.', children=[inner]), say(['traitor'], 'Top.')))
        self.assertIsNone(entries(say(['a'], 'Go {b}.', children=[say(['b'], 'Then {c}.', children=[child]),
                                                                    say(['c'], 'Back.')])))

    def test_a_nested_node_is_named_by_its_key_path(self):
        parent = say(['a'], 'Go.', children=[say(['b'], 'Then {nowhere}.')])
        self.assertEqual(entries(parent), [{'node': 'a/b', 'link': 'nowhere', 'reason': 'NO_HANDLER'}])

    def test_lua_pattern_keywords_are_evaluated(self):
        self.assertIsNone(entries(say(['a'], 'See {citizens}.'), say(['citizens?'], 'Yes.')))
        self.assertIsNone(entries(say(['a'], 'See {citizens}.'), say(['^citi'], 'Yes.')))
        self.assertEqual(len(entries(say(['a'], 'See {citizens}.'), say(['^zen'], 'No.'))), 1)

    def test_percent_z_is_nul_and_percent_capital_z_is_any_non_nul(self):
        # %z matches only NUL, never the letter z; %Z matches every other character, including z
        self.assertEqual(len(entries(say(['a'], 'See {zz}.'), say(['%z'], 'No.'))), 1)
        self.assertIsNone(entries(say(['a'], 'See {zz}.'), say(['^%Z%Z$'], 'Yes.')))
        self.assertEqual(len(entries(say(['a'], 'See {x}.'), say(['%Z%Z'], 'No.'))), 1)
        self.assertEqual(len(entries(say(['a'], 'See {x}.'), say(['[%z]'], 'No.'))), 1)
        self.assertIsNone(entries(say(['a'], 'See {x}.'), say(['[%Z]'], 'Yes.')))

    def test_a_keyword_the_stage_cannot_evaluate_stops_the_stage(self):
        for keyword in ('(captured)', '%bxy', '%f[a]', '[unclosed', '%'):
            with self.assertRaises(stage.StageError, msg=keyword):
                entries(say(['a'], 'See {x}.'), say([keyword], 'Yes.'))

    # Expected results come from running string.find on the pinned LuaJIT; 'ERR' is a Lua pattern
    # error, which the stage may only mirror by stopping (LuaPatternError), never by answering.
    LUAJIT_FIND = (
        ('a', '%a', 'Y'),
        ('1', '%a', 'N'),
        (' ', '%a', 'N'),
        ('.', '%a', 'N'),
        ('A', '%A', 'N'),
        ('!', '%A', 'Y'),
        ('\x01', '%a', 'N'),
        ('a', '%c', 'N'),
        ('1', '%c', 'N'),
        (' ', '%c', 'N'),
        ('.', '%c', 'N'),
        ('A', '%C', 'Y'),
        ('!', '%C', 'Y'),
        ('\x01', '%c', 'Y'),
        ('a', '%d', 'N'),
        ('1', '%d', 'Y'),
        (' ', '%d', 'N'),
        ('.', '%d', 'N'),
        ('A', '%D', 'Y'),
        ('!', '%D', 'Y'),
        ('\x01', '%d', 'N'),
        ('a', '%g', 'Y'),
        ('1', '%g', 'Y'),
        (' ', '%g', 'N'),
        ('.', '%g', 'Y'),
        ('A', '%G', 'N'),
        ('!', '%G', 'N'),
        ('\x01', '%g', 'N'),
        ('a', '%l', 'Y'),
        ('1', '%l', 'N'),
        (' ', '%l', 'N'),
        ('.', '%l', 'N'),
        ('A', '%L', 'Y'),
        ('!', '%L', 'Y'),
        ('\x01', '%l', 'N'),
        ('a', '%p', 'N'),
        ('1', '%p', 'N'),
        (' ', '%p', 'N'),
        ('.', '%p', 'Y'),
        ('A', '%P', 'Y'),
        ('!', '%P', 'N'),
        ('\x01', '%p', 'N'),
        ('a', '%s', 'N'),
        ('1', '%s', 'N'),
        (' ', '%s', 'Y'),
        ('.', '%s', 'N'),
        ('A', '%S', 'Y'),
        ('!', '%S', 'Y'),
        ('\x01', '%s', 'N'),
        ('a', '%u', 'N'),
        ('1', '%u', 'N'),
        (' ', '%u', 'N'),
        ('.', '%u', 'N'),
        ('A', '%U', 'N'),
        ('!', '%U', 'Y'),
        ('\x01', '%u', 'N'),
        ('a', '%w', 'Y'),
        ('1', '%w', 'Y'),
        (' ', '%w', 'N'),
        ('.', '%w', 'N'),
        ('A', '%W', 'N'),
        ('!', '%W', 'Y'),
        ('\x01', '%w', 'N'),
        ('a', '%x', 'Y'),
        ('1', '%x', 'Y'),
        (' ', '%x', 'N'),
        ('.', '%x', 'N'),
        ('A', '%X', 'N'),
        ('!', '%X', 'Y'),
        ('\x01', '%x', 'N'),
        ('z', '%z', 'N'),
        ('z', '%Z', 'Y'),
        ('\x00', '%z', 'Y'),
        ('a', '%0', 'ERR'),
        ('a', '%1', 'ERR'),
        ('ab', '^a', 'Y'),
        ('ab', '^b', 'N'),
        ('ab', 'b$', 'Y'),
        ('ab', 'a$', 'N'),
        ('a^b', 'a^b', 'Y'),
        ('a$b', 'a$b', 'Y'),
        ('ab', '^ab$', 'Y'),
        ('aaa', '^a-$', 'Y'),
        ('b', 'a-b', 'Y'),
        ('b', 'a*b', 'Y'),
        ('b', 'a+b', 'N'),
        ('b', 'ab?', 'N'),
        ('ab', 'ab?c', 'N'),
        ('a.c', 'a.c', 'Y'),
        ('b', '[a-c]', 'Y'),
        ('d', '[a-c]', 'N'),
        ('d', '[^a-c]', 'Y'),
        (']', '[]a]', 'Y'),
        ('b', '[^]]', 'Y'),
        ('-', '[a-]', 'Y'),
        ('-', '[%a-]', 'Y'),
        ('5', '[%d-]', 'Y'),
        ('a', '[a', 'ERR'),
        ('a', 'a%', 'ERR'),
        ('a', '%', 'ERR'),
        ('a)', 'a)', 'Y'),
        ('a]', 'a]', 'Y'),
        ('a)', '%)', 'Y'),
        ('(', '(', 'ERR'),
        ('a', 'a)', 'N'),
        ('a', '[%]]', 'N'),
        (']', '[%]]', 'Y'),
        ('a b', 'a b', 'Y'),
        ('', '', 'Y'),
        ('a', '', 'Y'),
    )

    def test_lua_find_agrees_with_luajit_or_stops(self):
        for text, pattern, expected in self.LUAJIT_FIND:
            with self.subTest(text=text, pattern=pattern):
                if expected == 'ERR':
                    with self.assertRaises(stage.LuaPatternError):
                        stage.lua_find(text, pattern)
                else:
                    self.assertEqual(stage.lua_find(text, pattern), expected == 'Y')

    def test_lua_find_stops_where_it_cannot_evaluate(self):
        for pattern in ('(a)', '%bxy', '%f[a]', '%0', '%1', '%9', '[a', '%'):
            with self.subTest(pattern=pattern):
                with self.assertRaises(stage.LuaPatternError):
                    stage.lua_find('xy', pattern)

    def test_lua_find_takes_the_plain_substring_path_without_specials(self):
        self.assertTrue(stage.lua_find('a)b', ')'))
        self.assertTrue(stage.lua_find('a]b', ']'))
        self.assertFalse(stage.lua_find('ab', ')'))

    def test_lua_find_depth_limit_matches_luajit(self):
        self.assertTrue(stage.lua_find('a' * 199, '^' + 'a?' * 199 + '$'))
        for count in (200, 250):
            with self.assertRaises(stage.LuaPatternError):
                stage.lua_find('a' * count, '^' + 'a?' * count + '$')

    def test_no_unanswered_link_leaves_the_field_absent(self):
        result = dialogue(say(['name'], 'Doctor Marrow.'))
        self.assertNotIn('source_incomplete', result)
        self.assertNotIn('links', result['keywords'][0])

    def test_dropping_the_links_or_the_entry_fails(self):
        result = dialogue(*marrow_chain())
        self.assertEqual(len(result['source_incomplete']), 2)
        stopped = next(node for node in result['keywords'] if node['key'] == 'stopped')
        self.assertTrue(stopped.get('links'))
        stripped = dict(stopped)
        stripped.pop('links')
        self.assertNotEqual(stopped, stripped)
        declaration = stage.build_declaration('doctor_marrow', result)
        self.assertEqual([e['link'] for e in declaration['source_incomplete']], ['helping', 'traitor'])

    def test_a_source_incomplete_difference_shows_in_the_conflict_diff(self):
        with_entry = dialogue(*marrow_chain())
        without = {key: value for key, value in with_entry.items() if key != 'source_incomplete'}
        self.assertEqual(stage.diff_fields(with_entry, without), ['source_incomplete'])


if __name__ == '__main__':
    unittest.main()
