"""Guarded, lossless correction and source-only association regression tests."""

import copy
import json
import unittest

import npc_corrective_overlay as overlay


def declaration(kind, key, **values):
    return {
        "kind": kind,
        "identity": {"key": key, "revision": "definition-r1"},
        "fields": [],
        **values,
    }


class CorrectiveOverlayTests(unittest.TestCase):
    def setUp(self):
        self.npc = declaration(
            "NPC",
            "oteryn:npc.hagor",
            presentation={
                "family": "Presentation",
                "key": "oteryn:presentation.npc.hagor",
                "revision": "definition-r1",
            },
            behavior={
                "family": "Behavior",
                "key": "oteryn:behavior.hagor",
                "revision": "definition-r1",
            },
            dialogue=None,
            services=[],
        )
        self.good = {
            "item": {
                "family": "Item",
                "key": "oteryn:item.a",
                "revision": "definition-r1",
            },
            "direction": "SellToPlayer",
            "unit_price": 10,
        }
        self.bad = {**self.good, "count": 7}
        self.trade = declaration(
            "Service", "oteryn:service.trade.hagor", offers=[self.good, self.bad]
        )
        self.route = {
            "key": "thais",
            "price": 10,
            "premium": False,
            "destination": {"coordinate_frame": "world", "x": 1, "y": 2, "floor": 7},
        }
        self.travel = declaration(
            "Service", "oteryn:service.travel.hagor", routes=[self.route]
        )
        self.dialogue = declaration(
            "Dialogue",
            "oteryn:dialogue.npc.hagor",
            greet=["hello"],
            keywords=[
                {
                    "key": "job",
                    "triggers": ["job"],
                    "reply": ["old"],
                    "children": [
                        {"key": "yes", "triggers": ["yes"], "reply": ["keep"]}
                    ],
                }
            ],
        )
        self.other = declaration(
            "Item", "oteryn:item.unrelated", unknown={"nested": [1, 2]}
        )
        self.baseline = {
            "schema": "example",
            "metadata": {"keep": True},
            "records": [self.npc, self.trade, self.travel, self.dialogue, self.other],
        }
        hold = {
            "old_native_offer": self.bad,
            "native_mapping": None,
            "source_proof": {"sha256": "a" * 64, "raw": "fluid=7"},
            "runtime_qualified": False,
        }
        corrected_trade = copy.deepcopy(self.trade)
        corrected_trade["offers"] = [self.good]
        corrected_trade["fields"] = [
            overlay.field("oteryn:source.npc.held_offer_000", hold)
        ]
        corrected_travel = copy.deepcopy(self.travel)
        corrected_travel["routes"][0]["price"] = 11
        corrected_dialogue = copy.deepcopy(self.dialogue)
        corrected_dialogue["keywords"][0]["reply"] = ["new"]
        self.inputs = {
            "r4": {"records": [corrected_trade]},
            "trade": {"services": []},
            "transport": {
                "services": [corrected_travel],
                "held_routes": [
                    {
                        "npc": self.npc["identity"]["key"],
                        "reason": "GATED_ROUTE",
                        "source_constraints": {"raw": "quest callback"},
                        "runtime_qualified": False,
                    }
                ],
            },
            "inventory": {
                "presentation_corrections": [
                    {
                        "npc_key": self.npc["identity"]["key"],
                        "source_reference": self.npc["presentation"],
                        "replacement_appearance": None,
                        "known_problem": "palette out of range",
                        "source_proofs": [{"sha256": "b" * 64}],
                        "corrected_declaration": {"ignored": True},
                    }
                ],
                "proposed_records": [declaration("NPC", "oteryn:npc.do_not_allocate")],
            },
            "static": {
                "records": [
                    {
                        "native_declaration": corrected_dialogue,
                        "classification": "PROVEN",
                        "active_dialogue_sha256": overlay.digest(
                            overlay.encode(self.dialogue)
                        ),
                        "changes": [
                            {
                                "pointer": "/keywords/0/reply",
                                "kind": "STATIC_REPLY_REPLACEMENT",
                                "before": ["old"],
                                "after": ["new"],
                                "proof": {"source_raw_sha256": "c" * 64},
                            }
                        ],
                    }
                ]
            },
            "quest": {
                "runtime_eligible": False,
                "native_quest_declarations": [],
                "records": [{"npc_key": self.npc["identity"]["key"]}],
            },
            "custody": {
                "quest": {"locator": "evidence/quest-stage.json", "sha256": "d" * 64}
            },
        }

    def plan(self):
        return overlay.build_plan(self.baseline, **self.inputs)

    def corrected(self):
        return overlay.apply_plan(self.baseline, self.plan())

    def test_preserves_unrelated_records_fields_and_structure(self):
        incoming = copy.deepcopy(self.baseline)
        incoming["records"][0]["fields"].append(
            overlay.field("unrelated", {"keep": True})
        )
        result = overlay.apply_plan(incoming, self.plan())
        self.assertEqual(result["metadata"], incoming["metadata"])
        self.assertEqual(result["records"][-1], self.other)
        self.assertEqual(result["records"][0]["behavior"], self.npc["behavior"])
        self.assertIn(
            incoming["records"][0]["fields"][0], result["records"][0]["fields"]
        )
        self.assertEqual(
            result["records"][3]["keywords"][0]["children"],
            self.dialogue["keywords"][0]["children"],
        )
        self.assertEqual(len(result["records"]), len(self.baseline["records"]))

    def test_idempotent_bytes_and_does_not_mutate_inputs(self):
        before = copy.deepcopy(self.baseline)
        plan = self.plan()
        first = overlay.apply_plan(before, plan)
        second = overlay.apply_plan(first, plan)
        self.assertEqual(overlay.encode(first), overlay.encode(second))
        self.assertEqual(before, self.baseline)

    def test_source_only_quest_and_raw_hold_proofs_retained(self):
        rows = overlay.index(self.corrected()["records"])
        npc = rows[self.npc["identity"]["key"]]
        associations = overlay.fields(npc["fields"])
        quest = json.loads(
            associations["oteryn:source.npc.quest_bindings"]["value"]["value"]
        )
        self.assertFalse(quest["runtime_eligible"])
        self.assertEqual(quest["sha256"], "d" * 64)
        self.assertEqual(quest["record_pointer"], "/records/0")
        travel = json.loads(
            associations["oteryn:source.npc.travel.unadmitted_routes"]["value"]["value"]
        )
        self.assertEqual(travel["held_routes"], self.inputs["transport"]["held_routes"])
        fields = rows[self.trade["identity"]["key"]]["fields"]
        self.assertEqual(
            json.loads(fields[0]["value"]["value"])["old_native_offer"], self.bad
        )

    def test_rejects_changed_original_offer(self):
        incoming = copy.deepcopy(self.baseline)
        incoming["records"][1]["offers"][0]["unit_price"] = 999
        with self.assertRaisesRegex(overlay.OverlayError, "unexpected original fact"):
            overlay.apply_plan(incoming, self.plan())

    def test_rejects_changed_original_reply(self):
        incoming = copy.deepcopy(self.baseline)
        incoming["records"][3]["keywords"][0]["reply"] = ["different"]
        with self.assertRaisesRegex(overlay.OverlayError, "unexpected original reply"):
            overlay.apply_plan(incoming, self.plan())

    def test_rejects_source_field_collision(self):
        incoming = copy.deepcopy(self.baseline)
        incoming["records"][0]["fields"] = [
            overlay.field("oteryn:source.npc.quest_bindings", {"wrong": True})
        ]
        with self.assertRaisesRegex(overlay.OverlayError, "unexpected source field"):
            overlay.apply_plan(incoming, self.plan())

    def test_rejects_stale_dialogue_custody(self):
        self.inputs["static"]["records"][0]["active_dialogue_sha256"] = "0" * 64
        with self.assertRaisesRegex(overlay.OverlayError, "stale static Dialogue"):
            self.plan()

    def test_rejects_unqualified_alias_or_guard_changes(self):
        self.inputs["static"]["records"][0]["native_declaration"]["keywords"][0][
            "triggers"
        ].append("alias")
        with self.assertRaisesRegex(overlay.OverlayError, "unrelated Dialogue facts"):
            self.plan()

    def test_rejects_trade_addition_or_reprice(self):
        self.inputs["r4"]["records"][0]["offers"][0] = {**self.good, "unit_price": 999}
        with self.assertRaisesRegex(overlay.OverlayError, "adds or reprices"):
            self.plan()

    def test_rejects_offer_removal_without_custody_hold(self):
        self.inputs["r4"]["records"][0]["fields"] = []
        with self.assertRaisesRegex(overlay.OverlayError, "no exact custody hold"):
            self.plan()

    def test_rejects_missing_target_or_different_revision(self):
        self.inputs["r4"]["records"][0]["identity"]["revision"] = "new"
        with self.assertRaisesRegex(overlay.OverlayError, "revision changed"):
            self.plan()
        self.inputs["r4"]["records"][0]["identity"]["key"] = "missing"
        with self.assertRaisesRegex(overlay.OverlayError, "missing Service"):
            self.plan()

    def test_rejects_duplicate_declarations_and_fields(self):
        self.baseline["records"].append(copy.deepcopy(self.other))
        with self.assertRaisesRegex(overlay.OverlayError, "duplicate declaration"):
            self.plan()
        self.baseline["records"].pop()
        f = overlay.field("duplicate", {})
        self.inputs["r4"]["records"][0]["fields"].extend([f, f])
        with self.assertRaisesRegex(overlay.OverlayError, "duplicate field_path"):
            self.plan()

    def test_rejects_typed_runtime_quest(self):
        self.inputs["quest"]["runtime_eligible"] = True
        with self.assertRaisesRegex(overlay.OverlayError, "only source associations"):
            self.plan()

    def test_supplied_plan_cannot_mutate_item_or_quest(self):
        for kind in ("Item", "Quest"):
            with self.subTest(kind=kind):
                record = declaration(kind, "oteryn:item.unrelated")
                plan = {
                    "schema": "oteryn.npc-corrective-overlay.v1",
                    "records": [
                        {
                            "key": record["identity"]["key"],
                            "identity": record["identity"],
                            "kind": kind,
                            "values": {"unknown": {"before": None, "after": 42}},
                            "fields": [],
                            "pointers": [],
                        }
                    ],
                }
                with self.assertRaisesRegex(
                    overlay.OverlayError, "only NPC/Dialogue/Service"
                ):
                    overlay.apply_plan({"records": [record]}, plan)

    def test_supplied_plan_cannot_change_identity_kind_or_native_guards(self):
        for name in ("identity", "kind", "behavior", "services", "quest_guard"):
            with self.subTest(name=name):
                plan = self.plan()
                row = next(r for r in plan["records"] if r["kind"] == "NPC")
                row["values"][name] = {"before": self.npc.get(name), "after": "changed"}
                with self.assertRaisesRegex(
                    overlay.OverlayError, "unsupported native field"
                ):
                    overlay.apply_plan(self.baseline, plan)

    def test_supplied_plan_cannot_add_routes_or_change_destination(self):
        plan = self.plan()
        row = next(r for r in plan["records"] if "routes" in r["values"])
        row["values"]["routes"]["after"][0]["destination"]["x"] = 999
        with self.assertRaisesRegex(overlay.OverlayError, "existing route removals"):
            overlay.apply_plan(self.baseline, plan)

    def test_supplied_plan_cannot_write_non_source_typed_fields(self):
        plan = self.plan()
        row = next(r for r in plan["records"] if r["kind"] == "NPC")
        row["fields"].append(
            {
                "path": "runtime.quest.guard",
                "before": None,
                "after": overlay.field("runtime.quest.guard", {"active": True}),
            }
        )
        with self.assertRaisesRegex(
            overlay.OverlayError, "only typed NPC source evidence"
        ):
            overlay.apply_plan(self.baseline, plan)

    def test_idempotent_after_native_serde_omits_empty_service_array(self):
        self.inputs["r4"]["records"][0]["offers"] = []
        self.inputs["r4"]["records"][0]["fields"].append(
            overlay.field(
                "oteryn:source.npc.held_offer_001", {"old_native_offer": self.good}
            )
        )
        plan = self.plan()
        first = overlay.apply_plan(self.baseline, plan)
        canonical = copy.deepcopy(first)
        del canonical["records"][1]["offers"]
        self.assertEqual(overlay.apply_plan(canonical, plan), first)


if __name__ == "__main__":
    unittest.main()
