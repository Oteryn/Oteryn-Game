"""Nested restrictions retain source ceilings; unknown and conflicting data cannot vanish."""

import tempfile
import unittest
from pathlib import Path

import engine_items
from test_engine_items import load_fixture_sources


class ImbuementLimitTests(unittest.TestCase):
    def convert(self, engine, children):
        with tempfile.TemporaryDirectory() as directory:
            sources = load_fixture_sources(Path(directory), engine)
            xml = f'<items><item id="100" name="fixture"><attribute key="primarytype" value="valuables"/><attribute key="imbuementslot" value="2">{children}</attribute></item></items>'
            sources["items"] = engine_items.load_items_xml(xml)
            return engine_items.convert_item(sources, 100)

    def test_both_engines_keep_nested_ceiling_without_flattening(self):
        children = '<attribute key="life leech" value="3"/><attribute key="skillboost club" value="2"/>'
        for engine in ("crystal", "canary"):
            item, _, report = self.convert(engine, children)
            self.assertEqual(
                item["imbuement"],
                {
                    "slot_count": 2,
                    "allowed_family_max_tiers": [
                        {"family": "life_leech", "max_tier": 3},
                        {"family": "skillboost_club", "max_tier": 2},
                    ],
                },
            )
            self.assertEqual(
                report["field_status"]["imbuementslot.allowed_family_max_tiers"],
                "mapped",
            )

    def test_invalid_or_duplicate_child_is_a_blocker_not_partial_data(self):
        for children in [
            '<attribute key="unrecognized" value="3"/>',
            '<attribute key="life leech" value="10"/>',
            '<attribute key="life leech"/>',
            '<attribute key="life leech" value="2"/><attribute key="life leech" value="3"/>',
        ]:
            item, _, report = self.convert("crystal", children)
            self.assertNotIn("allowed_family_max_tiers", item["imbuement"])
            self.assertIn(
                "converter_missing:imbuementslot.allowed_family_max_tiers",
                report["blockers"],
            )

    def test_absent_children_are_unknown_not_an_empty_whitelist(self):
        item, _, report = self.convert("crystal", "")
        self.assertEqual(item["imbuement"], {"slot_count": 2})
        self.assertNotIn(
            "imbuementslot.allowed_family_max_tiers", report["field_status"]
        )


if __name__ == "__main__":
    unittest.main()
