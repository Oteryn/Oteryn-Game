"""The selected hunting itinerary must not add rewards to source task payouts."""

import unittest

import chosen_campaign_rewards


class ChosenCampaignRewardTests(unittest.TestCase):
    def test_hunting_completion_preserves_earned_reports_without_a_bonus(self):
        document = chosen_campaign_rewards.generate()[
            "content/quests/chosen-reward-callbacks.json"
        ]
        callback = next(
            row for row in document["callbacks"]
            if row["key"] == "oteryn:quest-reward/chosen/hunting-campaign"
        )
        self.assertIsNone(callback.get("experience"))
        self.assertIsNone(callback["give"])
        self.assertTrue(callback["completes"])
        self.assertEqual(callback["storage"], [{
            "track": "oteryn:quest-progress/chosen/killing_in_the_name_of/phase",
            "minimum": 10, "maximum": 10, "set": 11,
        }])
        self.assertEqual(callback["required_receipts"], [
            {"transition": f"oteryn:quest-transition/grizzly/report-boss-{boss}",
             "track": f"oteryn:quest-progress/storage/{storage}", "value": 3}
            for boss, storage in [(1, 34100), (3, 34102), (19, 34118)]
        ])


if __name__ == "__main__":
    unittest.main()
