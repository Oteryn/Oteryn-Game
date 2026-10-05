"""Travel source regressions: no action loss, no unconditional Postman price."""

import copy
import hashlib
import json
import unittest
from pathlib import Path

import travel_semantics as travel

ROOT = Path(__file__).resolve().parents[3]
PACKET = ROOT / "docs/agents/evidence/OTV2-20261001-npc-source-audit-r4"


def source(lua):
    return {
        "state": "EXACT_PINNED_SOURCE",
        "lua": lua,
        "sha256": hashlib.sha256(lua.encode()).hexdigest(),
        "repository": "example/source",
        "revision": "1" * 40,
        "path": "npc.lua",
    }


def route(**changes):
    row = {
        "destination": {"x": 1, "y": 2, "z": 7},
        "price": 110,
        "premium": False,
        "min_level": None,
        "gate": "NONE",
        "effect": "NONE",
        "discount": "postman",
    }
    row.update(changes)
    return row


class TravelSemanticsTests(unittest.TestCase):
    def test_extra_action_is_held_even_when_source_prices_agree(self):
        plain, scripted = route(), route(effect="LUA_ACTION")
        self.assertIsNone(travel.route_hold_reason(plain))
        self.assertEqual(travel.route_hold_reason(scripted), "SCRIPTED_ROUTE")
        self.assertNotEqual(
            travel.source_route_signature(plain),
            travel.source_route_signature(scripted),
        )

    def test_unknown_or_dynamic_semantics_fail_closed(self):
        row = route()
        del row["effect"]
        self.assertEqual(travel.route_hold_reason(row), "ROUTE_SEMANTICS_UNKNOWN")
        self.assertEqual(
            travel.route_hold_reason(route(destination=None)), "ROUTE_SEMANTICS_UNKNOWN"
        )
        self.assertEqual(
            travel.route_hold_reason(route(gate="LUA_PREDICATE")), "GATED_ROUTE"
        )
        self.assertNotEqual(
            travel.source_route_signature(route()),
            travel.source_route_signature(route(discount=None)),
        )

    def test_discount_keeps_base_and_floors_free_route(self):
        self.assertEqual(travel.discounted_fare(110, [10]), 100)
        self.assertEqual(travel.discounted_fare(0, [10]), 0)
        self.assertEqual(travel.discounted_fare(5, [10, 20]), 0)
        with self.assertRaises(ValueError):
            travel.discounted_fare(110, [-10])
        with self.assertRaises(ValueError):
            travel.discounted_fare(True, [10])

    def test_full_callback_and_multiple_keyword_branches_survive(self):
        lua = """-- addTravelKeyword("carlin", 0, Position(0,0,0))
local ignored = 'addTravelKeyword("carlin", 0, Position(0,0,0))'
addTravelKeyword("carlin", 110, Position(1,2,7), function(player)
 if player:getStorageValue(Storage.Quest.Postman.Mission01) == 1 then
  player:setStorageValue(Storage.Quest.Postman.Mission01, 2)
 end
end)
addTravelKeyword("carlin", 200, Position(3,4,7))
"""
        evidence = travel.source_call_evidence(source(lua), "carlin")
        self.assertEqual(len(evidence["calls"]), 2)
        self.assertIn("setStorageValue", evidence["calls"][0]["source_call"])
        self.assertEqual(
            evidence["calls"][0]["storage_symbols"], ["Storage.Quest.Postman.Mission01"]
        )
        self.assertIsNone(evidence["native_gate"])

    def test_alias_keeps_previous_destination_call(self):
        evidence = travel.source_call_evidence(
            source(
                'addTravelKeyword("femor hills", 60, Position(1,2,7))\nkeywordHandler:addAliasKeyword({"hills"})'
            ),
            "hills",
        )
        self.assertEqual(len(evidence["calls"]), 1)
        self.assertIn(
            "femor hills", evidence["calls"][0]["source_previous_call"]["source_call"]
        )

    def test_tampered_pinned_capture_is_rejected(self):
        snapshot = source('addTravelKeyword("carlin",110,Position(1,2,7))')
        snapshot["lua"] += " "
        with self.assertRaisesRegex(ValueError, "digest"):
            travel.source_call_evidence(snapshot, "carlin")

    def load_inputs(self):
        def read(path):
            return json.loads(path.read_text())

        # The active package has already held three routes. Reconstruct the
        # correction from its retained original targets rather than reintroducing
        # those routes through an unqualified replay of historical R4 data.
        baseline = read(
            ROOT
            / "docs/agents/evidence/OTV2-20261001-npc-source-audit-r5/corrective-baseline.json"
        )
        original_services = [
            {"declaration": row}
            for row in baseline["records"]
            if row["kind"] == "Service"
            and row["identity"]["key"].startswith("oteryn:service.travel.")
        ]
        return (
            original_services,
            read(
                ROOT
                / "tools/content-schema/npc-authoring/samples/promotion-candidates-v1.json"
            )["candidates"],
            read(PACKET / "service-corrections.json")["records"],
        )

    def test_full_overlay_eight_fares_postman_custody_and_script_holds(self):
        inputs = self.load_inputs()
        original = copy.deepcopy(inputs)
        output = travel.repair_overlay(*inputs)
        self.assertEqual(inputs, original)
        self.assertEqual(output["counts"]["base_fare_changes"], 8)
        self.assertEqual(output["counts"]["conditional_discounts"], 171)
        self.assertEqual(output["counts"]["previous_held_routes"], 68)
        self.assertEqual(output["counts"]["scripted_route_holds"], 2)
        self.assertEqual(output["counts"]["routes_after_semantic_holds"], 195)
        breezelda = next(
            s
            for s in output["services"]
            if s["identity"]["key"].endswith(".captain_breezelda")
        )
        self.assertEqual(
            next(r["price"] for r in breezelda["routes"] if r["key"] == "carlin"), 110
        )
        self.assertEqual(
            next(
                d["conditional_price"]
                for d in output["conditional_discounts"]
                if d["service"].endswith(".captain_breezelda")
                and d["route"] == "carlin"
            ),
            100,
        )
        bluebear = next(
            s
            for s in output["services"]
            if s["identity"]["key"].endswith(".captain_bluebear")
        )
        self.assertNotIn("carlin", [r["key"] for r in bluebear["routes"]])
        self.assertEqual(output["counts"]["access_route_holds"], 1)
        self.assertEqual(
            output["access_route_holds"][0]["source_access"]["value"],
            "Rathleton Quest rank Citizen",
        )
        self.assertEqual(len(output["route_access_observations"]), 8)
        self.assertEqual(
            output["conditional_discounts"][0]["amount_source_proof"]["source_storage"],
            "Storage.Quest.ExampleQuest",
        )
        self.assertFalse(output["runtime_qualified"])
        self.assertFalse(output["global_complete"])
        self.assertEqual(output, travel.repair_overlay(*inputs))

    def test_discount_quote_cannot_replace_undiscounted_native_price(self):
        rows, candidates, corrections = self.load_inputs()
        breezelda = next(
            r
            for r in corrections
            if r["identity"]["key"].endswith(".captain_breezelda")
        )
        breezelda["routes"][0]["price"] = 100
        with self.assertRaisesRegex(ValueError, "undiscounted"):
            travel.repair_overlay(rows, candidates, corrections)

    def test_single_source_price_override_rejected(self):
        rows, candidates, corrections = self.load_inputs()
        breezelda = next(
            r
            for r in corrections
            if r["identity"]["key"].endswith(".captain_breezelda")
        )
        field = next(
            f for f in breezelda["fields"] if f["field_path"].endswith("price_quotes")
        )
        quotes = json.loads(field["value"]["value"])
        quotes["quotes"][0]["proof_refs"]["br"] = []
        field["value"]["value"] = json.dumps(quotes)
        with self.assertRaisesRegex(ValueError, "both pinned"):
            travel.repair_overlay(rows, candidates, corrections)


if __name__ == "__main__":
    unittest.main()
