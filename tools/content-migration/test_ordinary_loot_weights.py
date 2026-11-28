"""Crystal corpse loot retains Global item weights in the native provider."""

import json
import unittest
from decimal import Decimal
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


class OrdinaryLootWeights(unittest.TestCase):
    def test_lost_time_and_red_gem_keep_global_weight(self):
        capture = (
            ROOT / "docs/reference/content-prep/item-cyclopedia-capture-1/items.jsonl"
        )
        weights = {
            item["id"]: int(Decimal(item["basic"]["Weight"].removesuffix(" oz")) * 100)
            for line in capture.read_text().splitlines()
            if (item := json.loads(line))["id"] in (23729, 36706)
        }
        self.assertEqual(weights, {23729: 10, 36706: 100})
        profiles = ROOT / "content/items/definitions/spell-native-profiles.json"
        by_id = {
            int(row["production_binding"]["external_id"]): row
            for row in json.loads(profiles.read_bytes())["records"]
            if row["production_binding"]["external_id"] in ("23729", "36706")
        }
        for item, weight in weights.items():
            with self.subTest(item=item):
                physical = by_id[item]["semantics"]["physical"]["value"]
                self.assertEqual(
                    physical["weight"], {"state": "KNOWN", "value": weight}
                )
