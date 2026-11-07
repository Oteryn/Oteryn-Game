"""Comment and literal text must not produce quest storage write occurrences."""
import unittest
from lua_writers import scan


class WriterCommentFilterTests(unittest.TestCase):
    def test_short_long_comments_and_literals_keep_only_real_write(self):
        for prefix in ['-- player:setStorageValue(Storage.False, 2)',
                       '--[[player:setStorageValue(Storage.False, 2)]]',
                       'local s="player:setStorageValue(Storage.False, 2)"',
                       'local s=[=[player:setStorageValue(Storage.False, 2)]=]']:
            with self.subTest(prefix=prefix):
                text = prefix + '\nfunction x:onUse()\nplayer:setStorageValue(Storage.Live, 1)\nend'
                rows = scan(text, 'scripts/actions/test.lua')
                self.assertEqual([(r['target'], r['line'], r['to']) for r in rows], [('Storage.Live', 3, 1)])

    def test_trailing_comment_and_multiple_live_writes(self):
        text = ('function x:onUse()\n'
                'player:setStorageValue(Storage.Live, 1) -- player:setStorageValue(Storage.False, 2)\n'
                'player:setStorageValue(Storage.Live, 2)\nend')
        rows = scan(text, 'scripts/actions/test.lua')
        self.assertEqual([(r['target'], r['line'], r['to']) for r in rows],
                         [('Storage.Live', 2, 1), ('Storage.Live', 3, 2)])


if __name__ == '__main__':
    unittest.main()
