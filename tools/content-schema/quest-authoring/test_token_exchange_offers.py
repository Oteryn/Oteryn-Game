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

    def test_committed_exchange_precedence_and_quote_topics(self):
        offers = json.loads(
            (q.ROOT / "content/quests/shared-reward-offers.json").read_text()
        )["offers"]
        callbacks = json.loads((q.ROOT / q.token_exchanges.SHARD).read_text())[
            "callbacks"
        ]
        with self.subTest(npc="jorge"):
            self.assertEqual(
                [
                    row["keywords"][0]
                    for row in callbacks
                    if row["npc"] == "oteryn:npc.jorge"
                ],
                [name for name, *_ in q.token_exchanges.JORGE],
            )
        with self.subTest(npc="cledwyn"):
            self.assertEqual(
                [
                    row["keywords"][0]
                    for row in offers
                    if row["npc"] == "oteryn:npc.cledwyn"
                    and row.get("reward_order") == "before"
                ],
                ["token", "information", "talk", "enchant"],
            )
        with self.subTest(npc="storkus"):
            first = next(row for row in offers if row["npc"] == "oteryn:npc.storkus")
            self.assertEqual(first["keywords"], ["mission"])
            self.assertTrue(first.get("preserve_topic"))
            self.assertEqual(first.get("reward_order"), "before")
        gnomally = [row for row in offers if row["npc"] == "oteryn:npc.gnomally"]
        self.assertEqual(
            [
                row["keywords"][0]
                for row in gnomally
                if row.get("reward_order") == "before" and row.get("any_topic")
            ],
            ["equipment", "major", "minor", "relations", "relations", "items"],
        )
        for topic in range(101, 121):
            with self.subTest(npc="gnomally", topic=topic, action="replace"):
                self.assertTrue(
                    any(
                        row["next_topic"] == topic and row.get("any_topic")
                        for row in gnomally
                    ),
                    "cannot replace pending quote",
                )
            with self.subTest(npc="gnomally", topic=topic, action="cancel"):
                self.assertTrue(
                    any(
                        row["topic"] == topic
                        and row["keywords"] == ["no"]
                        and row["next_topic"] == 0
                        for row in gnomally
                    ),
                    "cannot cancel pending quote",
                )

    def test_committed_gnomally_relations_covers_unset_progress(self):
        offers = json.loads(
            (q.ROOT / "content/quests/shared-reward-offers.json").read_text()
        )["offers"]
        relations = [
            row
            for row in offers
            if row["npc"] == "oteryn:npc.gnomally" and row["keywords"] == ["relations"]
        ]
        track = relations[0]["requires"][0]["track"]
        rules = q.ROOT / "content/dialogues/quest-rules"
        tracks = json.loads((rules / "quest-batch3-a-bigfoot_burden.json").read_text())[
            "tracks"
        ]
        declaration = next(row for row in tracks if row["key"] == track)
        for value in (declaration["initial"], 24, 25, declaration["max"]):
            with self.subTest(quest_line=value):
                matching = [
                    row
                    for row in relations
                    if row["requires"][0]["minimum"]
                    <= value
                    <= row["requires"][0]["maximum"]
                ]
                self.assertEqual(len(matching), 1)
                self.assertEqual(matching[0].get("preserve_topic"), value < 25)

    def test_committed_gnomally_items_has_shop_confirmation(self):
        offers = json.loads(
            (q.ROOT / "content/quests/shared-reward-offers.json").read_text()
        )["offers"]
        rows = [row for row in offers if row["npc"] == "oteryn:npc.gnomally"]
        prompt = next(row for row in rows if row["keywords"] == ["items"])
        self.assertEqual(prompt["next_topic"], 5)
        for keyword in ("yes", "no"):
            with self.subTest(keyword=keyword):
                reply = next(
                    row
                    for row in rows
                    if row["topic"] == 5 and row["keywords"] == [keyword]
                )
                self.assertEqual(reply["next_topic"], 0)
                self.assertTrue(reply["msg_contains"])
                self.assertEqual(reply.get("open_shop", False), keyword == "yes")

    def test_committed_gnomally_retained_confirmation_flags(self):
        callbacks = json.loads(
            (q.ROOT / "content/quests/shared-reward-callbacks.json").read_text()
        )["callbacks"]
        retained = [
            row
            for row in callbacks
            if row["npc"] == "oteryn:npc.gnomally" and 101 <= row["topic"] <= 105
        ]
        self.assertEqual(
            sorted(row["topic"] for row in retained), list(range(101, 106))
        )
        for row in retained:
            with self.subTest(
                callback=row["key"], behavior="confirmation before cancellation"
            ):
                self.assertEqual(row["keywords"], ["yes"])
                self.assertTrue(row.get("msg_contains"))
            with self.subTest(
                callback=row["key"], behavior="failed purchase retires quote"
            ):
                self.assertTrue(row.get("clear_topic_on_refusal"))
