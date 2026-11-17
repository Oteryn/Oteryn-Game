"""The generated dialogue retains its source order and native quest track."""
import unittest
import engnpc2_player_name as author


class DreamCourtsNameTest(unittest.TestCase):
    def test_committed_programme_is_current(self):
        self.assertEqual(author.OUTPUT.read_text(), author.render())

    def test_source_name_branch_follows_confirmations(self):
        rules = author.document()['rules']
        self.assertEqual(rules[2]['topics'], [2])
        self.assertEqual(rules[2]['effects'], [dict(kind='set_topic', value=3)])
        self.assertEqual(rules[5]['topics'], [3])
        self.assertTrue(rules[5]['match_player_name'])
        self.assertEqual(rules[5]['exclude_keywords'], ['yes'])
        self.assertEqual(rules[5]['effects'][0], dict(kind='set', track=author.TRACK, value=1))
        self.assertEqual(len(rules[5]['reply']), 8)
