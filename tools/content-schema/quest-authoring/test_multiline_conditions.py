"""SOURCE importer regressions; no donor dialogue text or native vocabulary."""
import tempfile
import unittest
from pathlib import Path

import ots_interactions as oi


class MultilineConditionsTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.repo = Path(self.tmp.name)

    def script(self, body):
        text = ('local action = Action()\nfunction action.onUse(player, item, fromPosition, target, toPosition)\n'
                + body + '\nend\naction:id(300)\n')
        (self.repo / 'fixture.lua').write_text(text)
        return oi.Script('canary', self.repo, 'fixture.lua', {}, 'fixture')

    def test_six_storage_requirements_are_one_guard(self):
        clauses = [f'player:getStorageValue(Storage.Test.Boss{n}) >= 1' for n in range(1, 7)]
        s = self.script('if\n' + '\nand '.join(clauses) + '\nthen\nplayer:setStorageValue(Storage.Test.Access, 1)\nend')
        result = s.interactions()[0]
        guard = result['rules'][0]['branch'][0]['when']
        self.assertEqual(len(guard['all']), 6)
        self.assertEqual({term['quest_stage']['progress'] for term in guard['all']},
                         {f'canary:quest-progress/test/boss{n}' for n in range(1, 7)})
        self.assertEqual(result['unresolved'], [])
        self.assertEqual(s.condition_headers.keys(), {3})
        self.assertEqual(result['rules'][0]['branch'][0]['then'][0]['to'], 1)

    def test_parentheses_and_boolean_precedence(self):
        s = self.script('if\n(player:getLevel() >= 10 or\nitem.itemid == 300)\nand not (player:getLevel() < 2)\nthen\nplayer:setStorageValue(Storage.Test.Stage, 1)\nend')
        guard = s.interactions()[0]['rules'][0]['branch'][0]['when']
        self.assertIn('any', guard['all'][0])
        self.assertTrue(guard['all'][1]['negate'])

    def test_comment_and_quoted_then_do_not_end_a_header(self):
        s = self.script('if -- then\nplayer:getLevel() >= 10 -- then ignored\nand string.find("then", "end")\nthen\nplayer:setStorageValue(Storage.Test.Stage, 1)\nend')
        result = s.interactions()[0]
        self.assertEqual(len(s.condition_headers), 1)
        self.assertEqual(result['rules'][0]['branch'][0]['when'], {'unresolved': {'line': 3}})
        self.assertNotIn('statement outside the transcribed vocabulary', [u['reason'] for u in result['unresolved']])

    def test_multiline_elseif_keeps_branch_and_physical_write_line(self):
        s = self.script('if player:getLevel() < 5 then\nreturn false\nelseif\nplayer:getLevel() >= 10\nand item.itemid == 300\nthen\nplayer:setStorageValue(Storage.Test.Stage, 1)\nend')
        branches = s.interactions()[0]['rules'][0]['branch']
        self.assertEqual(len(branches), 2)
        self.assertEqual(len(branches[1]['when']['all']), 2)
        write_line = next(n for n, line in enumerate(s.lines, 1) if ':setStorageValue' in line)
        self.assertEqual(write_line, 9)
        self.assertIn(9, s.line_scopes)
        self.assertNotIn(8, s.line_scopes)

    def test_unresolved_expression_keeps_start_line(self):
        s = self.script('if\nunknownGuard()\nand player:getLevel() > 1\nthen\nplayer:setStorageValue(Storage.Test.Stage, 1)\nend')
        self.assertEqual(s.interactions()[0]['rules'][0]['branch'][0]['when'], {'unresolved': {'line': 3}})

    def test_leading_literal_keeps_header_offsets(self):
        s = self.script('if\n"fixture then" and player:getLevel() > 1 then\nreturn true\nend')
        self.assertEqual(s.condition_headers[3], 'if "fixture then" and player:getLevel() > 1 then')
        self.assertIn('unresolved', s.interactions()[0]['rules'][0]['branch'][0]['when'])

    def test_missing_then_does_not_consume_a_statement(self):
        s = self.script('if\nplayer:getLevel() > 1\nplayer:setStorageValue(Storage.Test.Stage, 1)\nend')
        self.assertEqual(s.condition_headers, {})
        self.assertEqual(s.block_lines, s.code_lines)

    def test_unbalanced_or_unterminated_headers_remain_original(self):
        for body in ('if\n(player:getLevel() > 1\nthen', 'if\nplayer:getLevel() > 1)\nthen',
                     'if\n"unterminated\nthen', 'if\nlocal hidden = 1\nthen'):
            with self.subTest(body=body):
                s = self.script(body)
                self.assertEqual(s.condition_headers, {})
                self.assertEqual(s.block_lines, s.code_lines)

    def test_single_line_headers_and_line_count_unchanged(self):
        s = self.script('if player:getLevel() > 1 then\nplayer:setStorageValue(Storage.Test.Stage, 1)\nend')
        self.assertEqual(s.condition_headers, {})
        self.assertEqual(len(s.block_lines), len(s.lines))

    def test_effectful_predicate_calls_not_swallowed_by_folding(self):
        for call in ('player:removeItem(300, 1)', 'player:setStorageValue(Storage.Test.Stage, 1)',
                     'Game.createMonster("fixture", Position(1, 2, 7))'):
            with self.subTest(call=call):
                s = self.script('if\n' + call + '\nthen\nreturn true\nend')
                self.assertEqual(s.condition_headers, {})
                self.assertEqual(s.block_lines, s.code_lines)

    def test_branch_local_does_not_leak_to_sibling_after_folding(self):
        s = self.script('if\nplayer:getLevel() > 1\nthen\nlocal limit = 10\nelse\nif\nplayer:getLevel() > limit\nthen\nplayer:setStorageValue(Storage.Test.Stage, 1)\nend\nend')
        result = s.interactions()[0]
        sibling = result['rules'][0]['otherwise'][0]['branch'][0]['when']
        self.assertIn('unresolved', sibling)


if __name__ == '__main__':
    unittest.main()
