"""Partial source leech facts must not acquire fabricated zero companions."""

import unittest

import engine_items
from test_engine_items import synthetic_sources


class LeechFieldTests(unittest.TestCase):
    def test_partial_percentages_and_explicit_zero_survive_both_engines(self):
        for engine in ("crystal", "canary"):
            for prefix, resource in (("life", "health"), ("mana", "mana")):
                for suffix in ("chance", "amount"):
                    for raw in ("300", "0"):
                        with self.subTest(
                            engine=engine, prefix=prefix, suffix=suffix, raw=raw
                        ):
                            field = f"{prefix}leech{suffix}"
                            sources = synthetic_sources(
                                engine,
                                {
                                    100: {
                                        "attrs": {
                                            "primarytype": "valuables",
                                            field: raw,
                                        },
                                        "flags": {},
                                    }
                                },
                            )
                            item, _, report = engine_items.convert_item(sources, 100)
                            expected = {
                                "resource": resource,
                                f"{suffix}_percent": {
                                    "numerator": int(raw) // 100,
                                    "denominator": 1,
                                },
                            }
                            self.assertEqual(item["modifiers"]["leech"], [expected])
                            self.assertEqual(report["field_status"][field], "mapped")
                            self.assertNotIn(
                                f"{prefix}leech{'amount' if suffix == 'chance' else 'chance'}",
                                report["field_status"],
                            )

    def test_absent_source_fields_do_not_create_modifier(self):
        self.assertNotIn("leech", engine_items.build_modifiers({}))

    def test_complete_pair_keeps_fractional_source_precision(self):
        self.assertEqual(
            engine_items.build_modifiers(
                {"lifeleechamount": "125", "lifeleechchance": "10000"}
            )["leech"],
            [
                {
                    "resource": "health",
                    "amount_percent": {"numerator": 5, "denominator": 4},
                    "chance_percent": {"numerator": 100, "denominator": 1},
                }
            ],
        )


if __name__ == "__main__":
    unittest.main()
