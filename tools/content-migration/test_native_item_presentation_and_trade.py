"""The published Item provider preserves independently qualified consumer facts."""

import json
import unittest

import native_item_consumer_facts as facts


class NativeItemConsumerFacts(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.qualified = json.loads((facts.ROOT / facts.PATH).read_bytes())
        cls.profiles = facts.published_items(
            lambda path: (facts.ROOT / path).read_bytes()
        )

    def test_published_loot_keeps_legacy_market_eligibility(self):
        missing = [
            key
            for key in self.qualified["marketable_items"]
            if not facts.marketable(self.profiles[key])
        ]
        self.assertEqual(missing, [])

    def test_published_corpse_door_and_container_admissions_have_display_names(self):
        actual = {
            key: facts.display_name(self.profiles[key])
            for key in self.qualified["display_names"]
        }
        self.assertEqual(actual, self.qualified["display_names"])
