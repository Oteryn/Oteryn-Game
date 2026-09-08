#!/usr/bin/env python3
from __future__ import annotations

import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("farm_export", HERE / "export.py")
assert SPEC and SPEC.loader
farm = importlib.util.module_from_spec(SPEC); SPEC.loader.exec_module(farm)
GEN = "creature-gameplay-profiles-v1:e417-census-v1"


def source() -> dict:
    partial = {"state": "PARTIAL", "reason_codes": ["SOURCE_SEMANTICS_PARTIAL"]}
    unsupported = {"state": "UNSUPPORTED", "reason_codes": ["NO_ACCEPTED_GAME_SOURCE"]}
    return {
        "source": {"repository": "Oteryn/Oteryn-Game", "revision": "a" * 40,
                   "semantic_digest": "sha256:" + "b" * 64, "generation": GEN},
        "capabilities": {
            "item_identity": copy.deepcopy(partial), "creature_identity": copy.deepcopy(partial),
            "loot_probability": copy.deepcopy(partial), "loot_quantity": copy.deepcopy(partial),
            "placement_supply": copy.deepcopy(unsupported), "tasks": copy.deepcopy(unsupported),
            "weekly": copy.deepcopy(unsupported), "respawn": copy.deepcopy(unsupported),
        },
        "creatures": [{"creature_id": "monster-entity:" + "1" * 32,
                       "display_name": "Test Dragon", "generation": GEN}],
        "items": [{"item_id": "oteryn:item.gold-coin", "display_name": "Gold Coin", "generation": GEN}],
        "loot_relations": [
            {"creature_id": "monster-entity:" + "1" * 32, "item_id": "oteryn:item.gold-coin",
             "item_display_name": "Gold Coin", "item_resolution_state": "RESOLVED",
             "probability": {"numerator": 800000, "denominator": 1000000,
                             "context": "STATIC_MIGRATION_PROFILE_NOT_LIVE_CURRENT"},
             "quantity": {"model": "BOUNDED_UNKNOWN", "min_count": 1, "max_count": 100},
             "generation": GEN},
            {"creature_id": "monster-entity:" + "1" * 32, "item_id": None,
             "item_display_name": "Unresolved Relic", "item_resolution_state": "UNRESOLVED",
             "probability": {"numerator": 1, "denominator": 10,
                             "context": "STATIC_MIGRATION_PROFILE_NOT_LIVE_CURRENT"},
             "quantity": {"model": "UNSUPPORTED"}, "generation": GEN},
        ],
    }


class ExportTests(unittest.TestCase):
    def assert_rejected(self, mutated: dict) -> None:
        with self.assertRaises(farm.ProductError): farm.build(mutated)

    def test_deterministic_order_bytes_and_digest(self) -> None:
        first = source(); second = copy.deepcopy(first)
        second["creatures"].reverse(); second["items"].reverse(); second["loot_relations"].reverse()
        self.assertEqual(farm.canonical_bytes(farm.build(first)), farm.canonical_bytes(farm.build(second)))
        self.assertEqual(farm.build(first)["semantic_digest"], farm.build(second)["semantic_digest"])

    def test_binding_and_unresolved_identity(self) -> None:
        product = farm.build(source())
        self.assertEqual(product["loot_relations"][0]["item_id"], None)
        self.assertEqual(product["loot_relations"][0]["item_resolution_state"], "UNRESOLVED")
        bad = source(); bad["loot_relations"][0]["creature_id"] = "monster-entity:" + "9" * 32
        self.assert_rejected(bad)
        bad = source(); bad["loot_relations"][0]["item_id"] = "oteryn:item.missing"
        self.assert_rejected(bad)

    def test_probability_range_denominator_and_context(self) -> None:
        for numerator, denominator in ((-1, 10), (11, 10), (1, 0), (1, farm.LIMITS["max_probability_denominator"] + 1)):
            bad = source(); bad["loot_relations"][0]["probability"].update(numerator=numerator, denominator=denominator)
            self.assert_rejected(bad)
        bad = source(); bad["loot_relations"][0]["probability"]["context"] = ""
        self.assert_rejected(bad)

    def test_quantity_models_and_zero_yield_exact_pmf(self) -> None:
        base = source()
        variants = [
            {"model": "FIXED", "count": 2},
            {"model": "EXACT_PMF", "pmf": [{"count": 0, "numerator": 1, "denominator": 1}]},
            {"model": "BOUNDED_UNKNOWN", "min_count": 1, "max_count": 8},
            {"model": "UNSUPPORTED"},
        ]
        for quantity in variants:
            value = copy.deepcopy(base); value["loot_relations"][0]["quantity"] = quantity
            if quantity["model"] in {"FIXED", "EXACT_PMF"}:
                value["capabilities"]["loot_quantity"] = {"state": "COMPLETE", "reason_codes": []}
            self.assertEqual(farm.build(value)["loot_relations"][1]["quantity"]["model"], quantity["model"])
        bad = source(); bad["loot_relations"][0]["quantity"] = {"model": "EXACT_PMF", "pmf": [{"count": 0, "numerator": 1, "denominator": 2}]}
        self.assert_rejected(bad)

    def test_duplicates_conflicts_and_generations(self) -> None:
        for family in ("creatures", "items", "loot_relations"):
            bad = source(); bad[family].append(copy.deepcopy(bad[family][0])); self.assert_rejected(bad)
        bad = source(); bad["creatures"][0]["generation"] = "other"; self.assert_rejected(bad)
        bad = source(); bad["source"]["semantic_digest"] = "wrong"; self.assert_rejected(bad)

    def test_capability_states_distinguish_empty_and_missing_proof(self) -> None:
        value = source(); value["creatures"] = []; value["items"] = []; value["loot_relations"] = []
        product = farm.build(value)
        self.assertEqual(product["capabilities"]["tasks"]["state"], "UNSUPPORTED")
        self.assertEqual(product["capabilities"]["loot_probability"]["state"], "PARTIAL")
        for state in ("UNKNOWN", "UNSUPPORTED", "PARTIAL"):
            candidate = source(); candidate["capabilities"]["tasks"] = {"state": state, "reason_codes": ["WHY"]}
            self.assertEqual(farm.build(candidate)["capabilities"]["tasks"]["state"], state)
        bad = source(); bad["capabilities"]["tasks"] = {"state": "UNSUPPORTED", "reason_codes": []}
        self.assert_rejected(bad)

    def test_malformed_corrupt_oversize_and_safe_output(self) -> None:
        product = farm.build(source()); product["creatures"][0]["display_name"] = "tampered"
        with self.assertRaises(farm.ProductError): farm.verify(product)
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "bad.json"; path.write_bytes(b"\xff")
            with self.assertRaises(farm.ProductError): farm.load(path, 10)
            path.write_text("x" * 11)
            with self.assertRaises(farm.ProductError): farm.load(path, 10)
        for path in ("../escape.json", "/absolute.json", "nested/out.json", "bad\\out.json"):
            with self.assertRaises(farm.ProductError): farm.safe_output(path)

    def test_no_dynamic_source_or_network_surface(self) -> None:
        text = (HERE / "export.py").read_text()
        for forbidden in ("exec(", "eval(", "requests", "urllib", "selenium", "playwright", ".lua", ".xml", ".otbm"):
            self.assertNotIn(forbidden, text)


if __name__ == "__main__": unittest.main()
