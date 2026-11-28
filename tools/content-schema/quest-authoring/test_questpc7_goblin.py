"""Source-bound Goblin introduction; no engine or quest-state fixture shortcuts."""

import copy
import json
import unittest

import qengine_rewards as sources
import questpc7_goblin_programme as goblin


class GoblinProgramme(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.inputs = sources.donor_inputs()

    def build(self, inputs=None):
        rows = []
        goblin.append(
            sources.ROOT,
            rows,
            self.inputs if inputs is None else inputs,
            sources.PIN,
            [],
        )
        return rows

    def test_start_initializes_all_six_jobs_atomically_without_free_stock(self):
        intro, *_ = self.build()
        self.assertEqual(len(intro["storage"]), 7)
        self.assertEqual(len({s["track"] for s in intro["storage"]}), 7)
        self.assertTrue(
            all(
                (s["minimum"], s["maximum"], s["set"]) == (-1, -1, 0)
                for s in intro["storage"]
            )
        )
        self.assertIsNone(intro["give"])
        self.assertNotIn("completes", intro)

    def test_bundle_preserves_source_prices_and_stack_class(self):
        _, kit, *_ = self.build()
        self.assertEqual(kit["fee"], {"gold": 1125, "include_bank": True})
        self.assertEqual(
            (kit["give"]["item"], kit["give"]["count"]), ("oteryn:item.tibia.i17668", 3)
        )
        self.assertEqual(
            {r["item"] for r in kit["give"]["backpack_outputs"]},
            {
                "oteryn:item.tibia.i17669",
                "oteryn:item.tibia.i17670",
                "oteryn:item.tibia.i17671",
                "oteryn:item.tibia.i17676",
            },
        )
        self.assertTrue(all(r["count"] == 1 for r in kit["give"]["backpack_outputs"]))
        self.assertTrue(
            any(
                s["track"].endswith("/kit_bought")
                and s["minimum"] == -1
                and s["maximum"] == -1
                and s["set"] == 1
                for s in kit["storage"]
            )
        )

    def test_changed_donor_price_refuses_the_authored_bundle(self):
        inputs = copy.copy(self.inputs)
        path = "data-global/npc/rafzan.lua"
        original = b"clientId = 17668, buy = 75"
        self.assertEqual(inputs[path].count(original), 1)
        inputs[path] = inputs[path].replace(original, b"clientId = 17668, buy = 74")
        with self.assertRaisesRegex(ValueError, "GOBLIN_SOURCE_TOOL_PRICE_DRIFT"):
            self.build(inputs)

    def test_six_source_reports_have_independent_cooldowns_and_pay(self):
        callbacks = {row["key"].rsplit("/", 1)[1]: row for row in self.build()}
        for topic, platinum, experience in (
            ("advertisement", 1, 600),
            ("perfume", 2, 1000),
            ("guards", 1, 600),
            ("busy", 3, 1200),
            ("destroy", 3, 1200),
            ("kill", 1, 600),
        ):
            with self.subTest(topic=topic):
                accept = callbacks["questpc7-goblin-accept-" + topic]
                (start_clock,) = accept["state_ops"]
                self.assertEqual(
                    start_clock["from"], {"kind": "elapsed_at_least", "seconds": 72000}
                )
                for suffix, lower, upper in (
                    ("", 0, 498),
                    ("-500th", 499, 499),
                    ("-after-500", 500, 999_999_999_999),
                ):
                    report = callbacks["questpc7-goblin-report-" + topic + suffix]
                    report_clock, tally = report["state_ops"]
                    self.assertEqual(start_clock["track"], report_clock["track"])
                    self.assertEqual(report_clock["effect"], {"kind": "set_now"})
                    self.assertEqual(
                        report["give"],
                        {
                            "item": "oteryn:item.tibia.i3035",
                            "count": platinum,
                            "contents": [],
                        },
                    )
                    self.assertEqual(report["experience"], experience)
                    self.assertEqual(
                        tally,
                        {
                            "track": goblin.TRACK + "reports",
                            "from": {
                                "kind": "between",
                                "minimum": lower,
                                "maximum": upper,
                            },
                            "effect": {"kind": "add", "value": 1},
                        },
                    )
                    self.assertEqual(
                        report.get("per_occurrence_mint", False), lower != 499
                    )
                    self.assertEqual(
                        report["achievement"],
                        "oteryn:achievement/task_manager" if lower == 499 else None,
                    )
        self.assertEqual(
            callbacks["questpc7-goblin-report-perfume"]["take"],
            [{"item": "oteryn:item.tibia.i17675", "count": 1}],
        )

    def test_guardcatcher_grants_can_repeat_until_the_third_capture_consumes_it(self):
        path = sources.ROOT / "content/quests/npc-callbacks/m13-goblin-capture.json"
        callbacks = json.loads(path.read_bytes())["callbacks"]
        first, second, third = [row for row in callbacks if row.get("capture_creature")]
        for step in (first, second):
            self.assertTrue(step["per_occurrence_mint"])
            self.assertEqual(step["give"]["item"], "oteryn:item.tibia.i17669")
        self.assertEqual(
            third["take"], [{"item": "oteryn:item.tibia.i17669", "count": 1}]
        )
        self.assertIsNone(third["give"])


if __name__ == "__main__":
    unittest.main()
