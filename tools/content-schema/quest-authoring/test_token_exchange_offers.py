"""Verify live-topic and keyword rules in the generated offer catalogue."""

import json
import unittest

import qengine_rewards as q


class TokenExchangeOffers(unittest.TestCase):
    def test_quote_cancellation_and_ordering_in_committed_offer_catalogue(self):
        offers = json.loads(
            (q.ROOT / "content/quests/shared-reward-offers.json").read_text()
        )["offers"]
        for name, products, first_topic in (
            ("cledwyn", q.token_exchanges.CLEDWYN, 101),
            ("gnomally", q.token_exchanges.GNOMALLY, 106),
        ):
            for topic in range(first_topic, first_topic + len(products)):
                with self.subTest(npc=name, topic=topic):
                    rows = [
                        row
                        for row in offers
                        if row["npc"] == f"oteryn:npc.{name}"
                        and row["keywords"] == ["no"]
                        and row["topic"] == topic
                    ]
                    self.assertEqual(len(rows), 1)
                    self.assertEqual(rows[0]["next_topic"], 0)

        for name, word, count, order in (
            ("cledwyn", "token", 1, "before"),
            ("gnomally", "major", 1, "before"),
            ("gnomally", "minor", 1, "before"),
            ("storkus", "vampire lord token", len(q.token_exchanges.RANKS), "before"),
            ("jorge", "souvenir", 1, "before"),
            ("ruprecht", "offers", 1, "alongside"),
        ):
            with self.subTest(npc=name, word=word):
                rows = [
                    row
                    for row in offers
                    if row["npc"] == f"oteryn:npc.{name}" and word in row["keywords"]
                ]
                self.assertEqual(len(rows), count)
                self.assertTrue(all(row.get("reward_order") == order for row in rows))
                if name == "ruprecht":
                    self.assertTrue(rows[0].get("preserve_topic"))

    def test_token_offer_keywords_and_live_topics(self):
        offers = json.loads(q.build()["content/quests/shared-reward-offers.json"])[
            "offers"
        ]
        expected = {
            "cledwyn": ("enchant",),
            "gnomally": tuple(row[0] for row in q.token_exchanges.GNOMALLY),
            "jorge": ("souvenir",),
            "ruprecht": ("offers", "token"),
        }
        for npc, words in expected.items():
            for word in words:
                with self.subTest(npc=npc, word=word):
                    offer = next(
                        row
                        for row in offers
                        if row["npc"] == f"oteryn:npc.{npc}"
                        and row["keywords"] == [word]
                    )
                    if word in {"enchant", "souvenir", "offers", "token"}:
                        self.assertTrue(offer.get("msg_contains"))
                    if word != "token":
                        self.assertTrue(offer.get("any_topic"))
