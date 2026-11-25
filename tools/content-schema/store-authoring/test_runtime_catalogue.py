import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import build_runtime_catalogue as runtime


class RuntimeCatalogueTests(unittest.TestCase):
    def test_committed_charm_entitlement_needs_no_item(self):
        self.assertEqual(runtime.check(), [])

    def test_charm_entitlement_still_requires_valid_price_and_currency(self):
        for field, value, message in (
            ("price", 0, "price or currency"),
            ("currency", "GOLD", "price or currency"),
            ("item", {"key": "unexpected"}, "entitlement cannot deliver an item"),
        ):
            with self.subTest(field=field), tempfile.TemporaryDirectory() as directory:
                catalogue = json.loads(runtime.OUT.read_text(encoding="utf-8"))
                offer = next(
                    row
                    for row in catalogue["offers"]
                    if row["key"] == "store:charm-expansion"
                )
                offer[field] = value
                path = Path(directory) / "catalogue.json"
                path.write_text(json.dumps(catalogue), encoding="utf-8")
                with patch.object(runtime, "OUT", path):
                    self.assertIn(f"store:charm-expansion: {message}", runtime.check())

    def test_platform_activation_keeps_premium_web_offers(self):
        # Platform's catalog_tests derives a valid Premium Time import this way.
        fixture = (
            runtime.ROOT
            / "docs/contracts/fixtures/store-price-list-v1/price-list.valid.json"
        )
        active = json.loads(
            fixture.read_text(encoding="utf-8").replace("ITEM", "PREMIUM_TIME")
        )
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory) / "active.json"
            base.write_text(json.dumps(active))
            with patch.object(
                runtime, "PLATFORM_OUT", Path(directory) / "combined.json"
            ):
                with patch(
                    "sys.argv",
                    ["build_runtime_catalogue.py", "--platform-base", str(base)],
                ):
                    self.assertEqual(runtime.main(), 0)
                imported = json.loads(runtime.PLATFORM_OUT.read_bytes())
        web_offers = [
            offer
            for offer in imported["offers"]
            if offer["purchasable"]
            and all(unit["unit_kind"] == "PREMIUM_TIME" for unit in offer["units"])
        ]
        self.assertEqual(web_offers, active["offers"])

    def test_price_refresh_retains_the_previous_combined_import(self):
        fixture = (
            runtime.ROOT
            / "docs/contracts/fixtures/store-price-list-v1/price-list.valid.json"
        )
        active = json.loads(
            fixture.read_text(encoding="utf-8").replace("ITEM", "PREMIUM_TIME")
        )
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory) / "active.json"
            combined = Path(directory) / "combined.json"
            retained = Path(directory) / "platform-predecessors.json"
            base.write_text(json.dumps(active))
            with (
                patch.object(runtime, "PLATFORM_OUT", combined),
                patch(
                    "sys.argv",
                    ["build_runtime_catalogue.py", "--platform-base", str(base)],
                ),
            ):
                self.assertEqual(runtime.main(), 0)
                original = combined.read_text()
                offer = active["offers"][0]
                offer["unit_price"] = str(int(offer["unit_price"]) + 1)
                base.write_text(json.dumps(active))
                self.assertEqual(runtime.main(), 0)
                self.assertNotEqual(combined.read_text(), original)
                self.assertIn(original, json.loads(retained.read_bytes()))

    def test_platform_only_creates_claims_for_physical_inbox_deliveries(self):
        catalogue = json.loads(runtime.OUT.read_bytes())
        prey = next(
            row
            for row in catalogue["offers"]
            if row["key"] == "store:prey-list-expansion"
        )
        charm = next(
            row for row in catalogue["offers"] if row["key"] == "store:charm-expansion"
        )
        prey.update(enabled=True, price=charm["price"])
        imported = {
            row["offer_key"]: row["units"][0]["unit_kind"]
            for row in json.loads(runtime.platform_catalogue(catalogue))["offers"]
        }
        for document in (
            catalogue,
            json.loads(runtime.OUT.with_name("hirelings.json").read_bytes()),
        ):
            for row in document["offers"]:
                with self.subTest(key=row["key"]):
                    # Platform creates CLAIM for each committed ITEM delivery.
                    self.assertEqual(imported[row["key"]] == "ITEM", "item" in row)

    def test_platform_import_matches_all_blessing_grants(self):
        ruleset = runtime.ROOT / "rulesets/character/blessings/catalogue.json"
        blessings = json.loads(ruleset.read_bytes())["blessings"]
        catalogue = json.loads(runtime.OUT.read_bytes())
        imported = {
            row["offer_key"]: row
            for row in json.loads(runtime.platform_catalogue(catalogue))["offers"]
        }
        for blessing in blessings:
            storage_key = blessing["storage_key"]
            key = "store:blessing:" + storage_key.removeprefix("oteryn:blessing.")
            with self.subTest(key=key):
                self.assertIn(key, imported)
                self.assertEqual(
                    imported[key],
                    {
                        "offer_key": key,
                        "currency": "COINS",
                        "unit_price": str(blessing["store_tibia_coins"]),
                        "max_quantity": 1,
                        "purchasable": True,
                        "units": [
                            {
                                "product_id": "oteryn.store.character_service",
                                "product_version": "1",
                                "unit_kind": "COSMETIC",
                                "grant_key": f"store-blessing:{storage_key}",
                                "grant_revision": "1",
                                "quantity": 1,
                            }
                        ],
                    },
                )
