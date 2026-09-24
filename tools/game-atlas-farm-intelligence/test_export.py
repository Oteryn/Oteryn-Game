#!/usr/bin/env python3
from __future__ import annotations

import copy
import importlib.util
from pathlib import Path
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("farm_export", HERE / "export.py")
assert SPEC and SPEC.loader
farm = importlib.util.module_from_spec(SPEC); SPEC.loader.exec_module(farm)


def fixture() -> dict:
    return {
        "test_authority": farm.TEST_AUTHORITY,
        "creatures": [{"creature_id": "monster-entity:" + "1" * 32, "display_name": "Test Dragon"}],
        "items": [{"item_id": "oteryn:item.gold-coin", "display_name": "Gold Coin"}],
        "loot_relations": [
            {"creature_id": "monster-entity:" + "1" * 32, "item_id": "oteryn:item.gold-coin",
             "item_display_name": "Gold Coin", "item_resolution_state": "RESOLVED",
             "probability": {"numerator": 8, "denominator": 10, "context": "TEST_ONLY_STATIC_NOT_AUTHORITY"},
             "quantity": {"model": "BOUNDED_UNKNOWN", "min_count": 1, "max_count": 100}},
            {"creature_id": "monster-entity:" + "1" * 32, "item_id": None,
             "item_display_name": "Unresolved Relic", "item_resolution_state": "UNRESOLVED",
             "probability": {"numerator": 1, "denominator": 10, "context": "TEST_ONLY_STATIC_NOT_AUTHORITY"},
             "quantity": {"model": "UNSUPPORTED"}},
        ],
    }


class ExportTests(unittest.TestCase):
    def assert_rejected(self, value: dict) -> None:
        with self.assertRaises(farm.ProductError):
            farm.build_test_fixture(value)

    def test_production_is_exact_fail_closed_product(self) -> None:
        product = farm.blocked_product()
        self.assertEqual(product["publication_state"], "BLOCKED_NO_ADMITTED_SOURCE")
        self.assertNotIn("limits", product)
        self.assertTrue(all(cap["state"] == "UNSUPPORTED" for cap in product["capabilities"].values()))
        farm.verify(product)
        bad = copy.deepcopy(product); bad["source"] = {"repository": "caller", "revision": "a" * 40}
        with self.assertRaises(farm.ProductError): farm.verify(bad)

    def test_cli_product_round_trip_and_exact_length_rejection(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "product.json"
            path.write_bytes(farm.canonical_bytes(farm.blocked_product()))
            farm.load_blocked_product(path)
            path.write_bytes(path.read_bytes() + b" ")
            with self.assertRaises(farm.ProductError): farm.load_blocked_product(path)

    def test_synthetic_fixture_cannot_masquerade_as_publication(self) -> None:
        artifact = farm.build_test_fixture(fixture())
        self.assertEqual(artifact["contract_id"], farm.TEST_CONTRACT_ID)
        self.assertEqual(artifact["publication_state"], "TEST_ONLY_NON_PUBLISHABLE")
        with self.assertRaises(farm.ProductError): farm.verify(artifact)
        bad = fixture(); bad["test_authority"] = "Oteryn/Oteryn-Game"
        self.assert_rejected(bad)

    def test_deterministic_fixture_order_digest_and_unresolved_identity(self) -> None:
        first = fixture(); second = copy.deepcopy(first)
        second["loot_relations"].reverse()
        self.assertEqual(farm.canonical_bytes(farm.build_test_fixture(first)), farm.canonical_bytes(farm.build_test_fixture(second)))
        product = farm.build_test_fixture(first)
        self.assertEqual(product["loot_relations"][0]["item_id"], None)
        self.assertEqual(product["loot_relations"][0]["item_resolution_state"], "UNRESOLVED")

    def test_synthetic_probability_and_relations_fail_closed(self) -> None:
        for numerator, denominator in ((-1, 10), (11, 10), (1, 0)):
            bad = fixture(); bad["loot_relations"][0]["probability"].update(numerator=numerator, denominator=denominator)
            self.assert_rejected(bad)
        bad = fixture(); bad["loot_relations"][0]["probability"]["context"] = "EXACT_RULESET_PROFILE_BASE"
        self.assert_rejected(bad)
        bad = fixture(); bad["loot_relations"][0]["item_id"] = "oteryn:item.missing"
        self.assert_rejected(bad)
        bad = fixture(); bad["loot_relations"].append(copy.deepcopy(bad["loot_relations"][0]))
        self.assert_rejected(bad)

    def test_synthetic_quantity_models_include_zero_yield_pmf(self) -> None:
        variants = [
            {"model": "FIXED", "count": 2},
            {"model": "EXACT_PMF", "pmf": [{"count": 0, "numerator": 1, "denominator": 1}]},
            {"model": "BOUNDED_UNKNOWN", "min_count": 1, "max_count": 8},
            {"model": "UNSUPPORTED"},
        ]
        for quantity in variants:
            value = fixture(); value["loot_relations"][0]["quantity"] = quantity
            self.assertEqual(farm.build_test_fixture(value)["loot_relations"][1]["quantity"]["model"], quantity["model"])
        bad = fixture(); bad["loot_relations"][0]["quantity"] = {"model": "EXACT_PMF", "pmf": [{"count": 0, "numerator": 1, "denominator": 2}]}
        self.assert_rejected(bad)

    def test_test_only_limits_are_not_public_contract_fields(self) -> None:
        self.assertNotIn("limits", farm.blocked_product())
        bad = fixture(); bad["creatures"][0]["display_name"] = "x" * (farm.TEST_ONLY_LIMITS["max_string_bytes"] + 1)
        self.assert_rejected(bad)

    def test_safe_paths_and_no_dynamic_source_or_network_surface(self) -> None:
        for path in ("../escape.json", "/absolute.json", "nested/out.json", "bad\\out.json"):
            with self.assertRaises(farm.ProductError): farm.safe_output(path)
        text = (HERE / "export.py").read_text()
        for forbidden in ("exec(", "eval(", "requests", "urllib", "selenium", "playwright", ".lua", ".xml", ".otbm"):
            self.assertNotIn(forbidden, text)

    def test_export_rejects_symlink_to_existing_outside_target(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            output_directory = directory / "output"
            output_directory.mkdir()
            outside = directory / "outside.json"
            outside.write_bytes(b"must remain unchanged")
            output = output_directory / "product.json"
            output.symlink_to(outside)

            with self.assertRaises(farm.ProductError):
                farm.write_blocked_product(output)
            self.assertEqual(outside.read_bytes(), b"must remain unchanged")

    def test_export_rejects_dangling_symlink_without_creating_target(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            output_directory = directory / "output"
            output_directory.mkdir()
            outside = directory / "missing-outside.json"
            output = output_directory / "product.json"
            output.symlink_to(outside)

            with self.assertRaises(farm.ProductError):
                farm.write_blocked_product(output)
            self.assertFalse(outside.exists())


if __name__ == "__main__": unittest.main()
