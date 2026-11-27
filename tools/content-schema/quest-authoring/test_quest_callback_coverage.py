"""Coverage prefers the test that claims all three Skull of Ratha rewards."""
import unittest

from quest_callback_coverage import QUALIFIED


class QuestCallbackCoverageTests(unittest.TestCase):
    def test_skull_of_ratha_claims_use_the_shared_witness(self):
        for claim in (
            "oteryn:reward-claim.kv.witch_house_quest",
            "oteryn:reward-claim.quest.u7_6.explorer_society.skull_of_ratha.bag1",
            "oteryn:reward-claim.quest.u7_6.explorer_society.skull_of_ratha.bag2",
        ):
            with self.subTest(claim=claim):
                self.assertEqual(
                    QUALIFIED[claim],
                    "qc_chests::qc_skull_of_ratha_three_source_chests_use_replay_and_refusal",
                )
