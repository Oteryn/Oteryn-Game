"""Exact offline source closure for nonmutating carried quest disguises."""
import unittest
from qengine_rewards import ROOT
import quest_item_disguises


class QuestItemDisguiseTests(unittest.TestCase):
    def test_wrath_source_retains_item_stage_and_has_exact_permanent_look_exit(self):
        rules = quest_item_disguises.build(ROOT)
        self.assertEqual(len(rules), 1)
        rule = rules[0]
        self.assertEqual((rule['item'], rule['appearance'], rule['required']),
                         ('oteryn:item.tibia.i11328', 11328, 2))
        self.assertEqual(rule['restore'], {'x':33323,'y':31080,'floor':-7})
        self.assertEqual(len(rule['sources']), 3)
        self.assertEqual((rule['use_effect'], rule['speech_type']), (3,36))
        self.assertNotIn('take', rule)
        self.assertNotIn('set', rule)


if __name__ == '__main__':
    unittest.main()
