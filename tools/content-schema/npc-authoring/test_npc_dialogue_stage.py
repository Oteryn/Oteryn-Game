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


def marrow_chain():
    return [say(['unpleasant'], 'It must be {stopped}.'), say(['stopped'], STOPPED),
            say(['name'], 'Doctor Marrow.')]


def cipfried_tibia():
    return say(['tibia'], "That's where we are: its {citizens}, its {merchants} and its {monsters}.")


class SourceIncompleteTests(unittest.TestCase):
    def test_doctor_marrow_stopped_keeps_its_link_and_records_one_entry(self):
        result = dialogue(*marrow_chain())
        stopped = next(node for node in result['keywords'] if node['key'] == 'stopped')
        self.assertEqual(stopped['links'], ['traitor'])
        self.assertEqual(result['source_incomplete'], [{'node': 'stopped', 'link': 'traitor', 'reason': 'NO_HANDLER'}])
        declaration = stage.build_declaration('doctor_marrow', result)
        self.assertEqual(declaration['source_incomplete'], result['source_incomplete'])

    def test_every_emitted_node_keeps_its_links(self):
        result = dialogue(*marrow_chain())
        self.assertEqual({node['key']: node.get('links') for node in result['keywords']},
                         {'unpleasant': ['stopped'], 'stopped': ['traitor'], 'name': None})

    def test_a_link_with_a_handler_adds_no_entry(self):
        nodes = marrow_chain() + [say(['traitor'], 'A traitor indeed.')]
        self.assertIsNone(entries(*nodes))
        self.assertNotIn('source_incomplete', stage.build_declaration('doctor_marrow', dialogue(*nodes)))

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
        result = dialogue(*(marrow_chain() + [gated]))
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

    def test_a_keyword_the_stage_cannot_evaluate_stops_the_stage(self):
        for keyword in ('(captured)', '%bxy', '%f[a]', '[unclosed', '%'):
            with self.assertRaises(stage.StageError, msg=keyword):
                entries(say(['a'], 'See {x}.'), say([keyword], 'Yes.'))

    def test_no_unanswered_link_leaves_the_field_absent(self):
        result = dialogue(say(['name'], 'Doctor Marrow.'))
        self.assertNotIn('source_incomplete', result)
        self.assertNotIn('links', result['keywords'][0])

    def test_dropping_the_links_or_the_entry_fails(self):
        result = dialogue(*marrow_chain())
        self.assertEqual(len(result['source_incomplete']), 1)
        stopped = next(node for node in result['keywords'] if node['key'] == 'stopped')
        self.assertTrue(stopped.get('links'))
        stripped = dict(stopped)
        stripped.pop('links')
        self.assertNotEqual(stopped, stripped)
        declaration = stage.build_declaration('doctor_marrow', result)
        self.assertEqual([e['link'] for e in declaration['source_incomplete']], ['traitor'])


if __name__ == '__main__':
    unittest.main()
