import unittest

from lower_dialogue import compact, split_boolean


class LiteralLoweringTests(unittest.TestCase):
    def test_compaction_preserves_keyword_spaces(self):
        self.assertEqual(compact('MsgContains(message, "animal cure")'), 'MsgContains(message,"animal cure")')

    def test_boolean_split_does_not_split_quoted_and(self):
        result = split_boolean('MsgContains(message, "food and fight") and value == 1', 'and')
        self.assertEqual(len(result), 2)
        self.assertIn('"food and fight"', result[0])


if __name__ == "__main__":
    unittest.main()
