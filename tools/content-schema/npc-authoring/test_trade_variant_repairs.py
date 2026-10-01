"""Transaction safety regressions for fluid variants and shop access callbacks."""

import copy
import hashlib
import unittest

import promotion_candidates as pc
import trade_variant_repair as repair
from test_promotion import make_bundle


def offer(item=2874, count=1, subtype=None, name="vial of water"):
    return {
        "client_id": item,
        "server_item_id": None,
        "item_name": name,
        "count": count,
        "sub_type": subtype,
        "buy_price": 40,
        "sell_price": None,
        "stock_gate": None,
    }


def native(item=2874, count=1, subtype=None):
    return {
        "item": {
            "family": "Item",
            "key": f"oteryn:item.tibia.i{item}",
            "revision": "definition-r1",
        },
        "direction": "SellToPlayer",
        "unit_price": 40,
        "count": count,
        **({"sub_type": subtype} if subtype is not None else {}),
    }


def service(name="ahmet", offers=None):
    return {
        "kind": "Service",
        "identity": {
            "key": "oteryn:service.trade." + name,
            "revision": "definition-r1",
        },
        "offers": offers if offers is not None else [native()],
        "fields": [],
    }


class TradeVariantRepairs(unittest.TestCase):
    def promote(self, name, source_offer):
        bundle = make_bundle(name)
        bundle["services"]["trade"] = {"currency": "GOLD", "offers": [source_offer]}
        itemmap = {
            "records": [
                {
                    "source_item_id": source_offer["client_id"],
                    "native_key": f"oteryn:item.tibia.i{source_offer['client_id']}",
                    "native_revision": "definition-r1",
                }
            ]
        }
        builder = pc.Builder({"npcs": [], "trade": {}}, itemmap)
        left = []
        rows = builder.merge_offers(
            {"canary": bundle, "crystal": copy.deepcopy(bundle)}, name, [], left
        )
        return rows, left

    def test_water_enum_one_does_not_become_generic_quantity(self):
        rows, left = self.promote("Ahmet", offer())
        self.assertEqual(rows, [])
        fluid = left[0]["source_semantics"]["fluid"][0]
        self.assertEqual(
            (fluid["source_fluid_name"], fluid["source_fluid_enum"]), ("water", 1)
        )
        self.assertEqual(fluid["old_source_offer"], offer())
        self.assertIsNone(fluid["native_sub_type"])

    def test_oil_enum_seven_is_held_with_exact_price_and_quantity(self):
        source = offer(count=7, name="vial of oil")
        rows, left = self.promote("Ahmet", source)
        self.assertEqual(rows, [])
        self.assertEqual(left[0]["source_semantics"]["offers"]["canary"], source)

    def test_waterskin_is_not_missed_by_water_hold(self):
        rows, left = self.promote("Ahmet", offer(item=2901, name="waterskin of water"))
        self.assertFalse(rows)
        self.assertTrue(left)
        result = repair.repair_trade_services([service(offers=[native(item=2901)])])
        self.assertEqual(result["services"][0]["offers"], [])

    def test_explicit_legacy_subtype_is_not_assumed_native(self):
        rows, left = self.promote(
            "Satsu", offer(count=1, subtype=3, name="vial of beer")
        )
        self.assertFalse(rows)
        self.assertTrue(left)
        result = repair.repair_trade_services([service("satsu", [native(subtype=3)])])
        self.assertEqual(
            result["counts"], {"SOURCE_FLUID_SUBTYPE_NATIVE_BINDING_UNQUALIFIED": 1}
        )

    def test_rune_quantity_and_plain_empty_container_are_preserved(self):
        rows, left = self.promote(
            "Sam", offer(item=3174, count=3, name="avalanche rune")
        )
        self.assertEqual((rows[0]["count"], left), (3, []))
        plain = native()
        plain.pop("count")
        result = repair.repair_trade_services([service(offers=[plain])])
        self.assertEqual(result["services"], [])

    def test_access_callback_held_even_when_item_stock_gate_empty(self):
        rows, left = self.promote(
            "Rashid", offer(item=3375, count=None, name="soldier helmet")
        )
        self.assertFalse(rows)
        predicate = left[0]["source_semantics"]["access"]["source_predicate"]
        self.assertEqual(
            predicate,
            {"storage": "TheTravellingTrader.Mission07", "operator": "==", "value": 1},
        )
        self.assertEqual(left[0]["reason"], "GATED_OFFER")

    def test_whole_shop_wiki_offer_cannot_reintroduce_access_held_item(self):
        builder = pc.Builder({"npcs": [], "trade": {}}, {"records": []})
        for name in ["Rashid", "Haroun", "Alesar", "Nah'Bob", "Yaman"]:
            with self.subTest(npc=name):
                self.assertEqual(builder.wiki_offers(name, [], [], []), [])

    def test_exact_source_capture_is_checked_and_preserved(self):
        raw = b'npcConfig.shop = {\n { itemName = "vial of water", clientId = 2874, buy = 40, count = 1 },\n}\n'
        source = {
            "repository": "opentibiabr/canary",
            "revision": repair.CANARY_REVISION,
            "sha256": hashlib.sha256(raw).hexdigest(),
            "path": "data-otservbr-global/npc/ahmet.lua",
        }
        quote = repair.source_quotes(raw, source)[0]
        self.assertEqual(quote["source_proof"]["line"], 2)
        self.assertIn("count = 1", quote["source_proof"]["raw_shop_row"])
        with self.assertRaisesRegex(ValueError, "SHA-256"):
            repair.source_quotes(raw + b" ", source)
        source["revision"] = "unbound"
        with self.assertRaisesRegex(ValueError, "source pin"):
            repair.source_quotes(raw, source)

    def test_symbolic_count_cannot_become_null_quantity_fact(self):
        raw = b'npcConfig.shop = {\n { itemName = "vial of water", clientId = 2874, buy = 40, count = FLUID_WATER },\n}\n'
        source = {
            "repository": "opentibiabr/canary",
            "revision": repair.CANARY_REVISION,
            "sha256": hashlib.sha256(raw).hexdigest(),
        }
        self.assertEqual(repair.source_quotes(raw, source), [])

    def test_native_repairs_preserve_currency_refs_and_are_idempotent(self):
        old = native(count=7)
        old["currency"] = {
            "family": "Item",
            "key": "oteryn:item.tibia.i22516",
            "revision": "definition-r1",
        }
        original = service(offers=[old])
        result = repair.repair_trade_services([original])
        self.assertEqual(result["held_offers"][0]["old_native_offer"], old)
        self.assertEqual(original["offers"], [old])
        self.assertEqual(
            repair.repair_trade_services(result["services"])["services"], []
        )
        self.assertEqual(result["services"][0]["offers"], [])

    def test_existing_variant_hold_keeps_complete_source_proof(self):
        old = native(item=50293, count=500)
        held = {
            "service": "oteryn:service.trade.sam",
            "old_native_offer": old,
            "reason": "VARIANT_SAME_TUPLE_TWO_WIKI_NOT_PROVEN",
            "source_proof": {"sha256": "a" * 64, "count_context": "charges"},
        }
        result = repair.repair_trade_services(
            [service("sam", [old])], retained_holds=[held]
        )
        self.assertEqual(result["held_offers"][0]["source_proof"], held["source_proof"])
        self.assertEqual(result["services"][0]["offers"], [])

    def test_native_access_hold_retains_item_rows_and_source_predicate(self):
        original = service("nah_bob", [native(item=3375, count=1)])
        result = repair.repair_trade_services([original])
        hold = result["held_offers"][0]
        self.assertEqual(
            hold["access_fact"]["source_predicate"]["storage"],
            "DjinnWar.MaridFaction.Mission03",
        )
        self.assertEqual(hold["old_native_offer"], original["offers"][0])
        self.assertIsNone(hold["access_fact"]["native_predicate"])
        self.assertEqual(result["services"][0]["offers"], [])


if __name__ == "__main__":
    unittest.main()
