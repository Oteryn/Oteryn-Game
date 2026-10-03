"""Typed source modifiers retain unknown context and reject whole-vector ambiguity."""

import copy
import unittest
from unittest.mock import patch

import lower_wiki_stats_packet as lower


class ModifierTests(unittest.TestCase):
    key = "oteryn:item.tibia.i1"

    def qualify(self, fields, *, definition=None, shared=False, others=()):
        observations = [
            {"page_id": 10 + i, "revision_id": 20, "fields": value}
            for i, value in enumerate([fields, *others])
        ]
        records = {self.key: {"item_id": 1, "observations": observations}}
        if shared:
            records["unbound"] = {
                "item_id": 999,
                "observations": [{"page_id": 10, "revision_id": 20, "fields": {}}],
            }
        definition = (
            definition
            if definition is not None
            else {"semantics": {"skill_modifiers": {"state": "UNKNOWN"}}}
        )
        rows, report, _ = lower.build({"records": records}, {1}, {self.key: definition})
        return [
            r for r in rows if r["field_path"] == "skill_modifiers.modifiers"
        ], report["physical_field_holds"]

    def test_points_percent_units_order_and_absent_companion_unknown(self):
        rows, holds = self.qualify(
            {
                "attrib": "speed -2, magic level +1",
                "manaleech_am": "0.125%",
                "hpleech_ch": "100%",
            }
        )
        self.assertEqual(holds, [])
        entries = rows[0]["typed_value"]["value"]
        self.assertEqual(
            [e["kind"] for e in entries],
            ["LIFE_LEECH_CHANCE", "MAGIC_LEVEL_POINTS", "MANA_LEECH_AMOUNT", "SPEED"],
        )
        self.assertEqual(
            entries[2]["parameter"],
            lower.known(
                {
                    "kind": "RATIONAL_PERCENT",
                    "value": {"numerator": 1, "denominator": 8},
                }
            ),
        )
        self.assertEqual(
            entries[3]["parameter"], lower.known({"kind": "SIGNED_POINTS", "value": -2})
        )
        for entry in entries:
            for field in ("target_domain", "evaluation_phase", "priority"):
                self.assertEqual(entry[field], {"state": "UNKNOWN"})
        self.assertNotIn("LIFE_LEECH_AMOUNT", [e["kind"] for e in entries])
        _, zero = lower.modifiers({"manaleech_am": "0%"})
        self.assertEqual(
            zero["value"][0]["parameter"]["value"]["value"],
            {"numerator": 0, "denominator": 1},
        )

    def test_unsupported_partial_duplicate_units_bounds_and_variants_hold_vector(self):
        for fields in (
            {"attrib": "magic level +1, faster regeneration"},
            {"attrib": "sword fighting +1, sword fighting +2"},
            {"attrib": "speed +2147483648"},
            {"manaleech_am": "1"},
            {"manaleech_am": "101%"},
            {"hpleech_am": "0.00000000000000000001%"},
        ):
            with self.subTest(fields=fields):
                rows, holds = self.qualify(fields)
                self.assertEqual(rows, [])
                self.assertEqual(holds[0]["reason"], "MALFORMED_WIKI_VALUE")
        rows, holds = self.qualify({"attrib": "magic level +1"}, shared=True)
        self.assertEqual(rows, [])
        self.assertEqual(holds[0]["sources"][0]["page_item_ids"], [1, 999])
        self.assertEqual(self.qualify({}), ([], []))
        rows, holds = self.qualify(
            {"attrib": "magic level +1"}, others=({"attrib": "magic level +2"},)
        )
        self.assertEqual(rows, [])
        self.assertEqual(holds[0]["reason"], "WIKI_PAGE_DISAGREEMENT")

    def test_explicit_unsupported_same_group_field_holds_entire_vector(self):
        for field, value in (("mantra", "12"), ("elementalbond", "physical")):
            for raw in ({field: value}, {"attrib": "fist fighting +1", field: value}):
                rows, holds = self.qualify(raw)
                self.assertEqual(rows, [])
                self.assertEqual(holds[0]["reason"], "MALFORMED_WIKI_VALUE")
                self.assertEqual(holds[0]["sources"][0]["values"], raw)
        rows, holds = self.qualify(
            {"attrib": "fist fighting +1"}, others=({"mantra": "12"},)
        )
        self.assertEqual(rows, [])
        self.assertEqual(len(holds[0]["sources"]), 2)
        self.assertEqual(
            self.qualify({"augments": "opaque Ability identity"}), ([], [])
        )

    def test_known_vector_idempotence_and_blocked_or_conflicting_values_preserved(self):
        fields = {"attrib": "magic level +1"}
        rows, _ = self.qualify(fields)
        value = rows[0]["typed_value"]["value"]
        definition = {
            "semantics": {
                "skill_modifiers": lower.known({"modifiers": lower.known(value)})
            }
        }
        self.assertEqual(
            self.qualify(fields, definition=definition), self.qualify(fields)
        )
        for name in ("target_domain", "evaluation_phase", "priority", "parameter"):
            bad = copy.deepcopy(definition)
            bad["semantics"]["skill_modifiers"]["value"]["modifiers"]["value"][0][
                name
            ] = {"state": "CONFLICT"}
            self.assertEqual(
                self.qualify(fields, definition=bad)[1][0]["reason"],
                "BLOCKED_EVIDENCE_STATE",
            )
        bad = copy.deepcopy(definition)
        bad["semantics"]["skill_modifiers"]["value"]["modifiers"]["value"][0][
            "parameter"
        ]["value"]["value"] = 2
        self.assertEqual(
            self.qualify(fields, definition=bad)[1][0]["reason"], "KNOWN_FIELD_CONFLICT"
        )

    def test_external_soulcrusher_hold_is_bound_to_exact_retained_facts(self):
        hold = lower.source_hold(
            lower.MODIFIER_SOURCE_HOLD, lower.MODIFIER_SOURCE_HOLD_SHA256
        )
        source = hold["expected_snapshot_sources"][0]
        obs = {k: v for k, v in source.items() if k != "values"}
        obs["fields"] = source["values"]
        key = hold["item_key"]
        records = {key: {"item_id": 34086, "observations": [obs]}}
        defs = {key: {"semantics": {"skill_modifiers": {"state": "UNKNOWN"}}}}
        rows, report, _ = lower.build({"records": records}, {34086}, defs)
        self.assertEqual(rows, [])
        self.assertEqual(
            report["physical_field_holds"][0]["classification"], "CONFLICT"
        )
        obs["fields"] = dict(obs["fields"], hpleech_am="5%")
        with self.assertRaisesRegex(ValueError, "snapshot facts drift"):
            lower.build({"records": records}, {34086}, defs)
        with (
            patch.object(lower, "MODIFIER_SOURCE_HOLD_SHA256", "0" * 64),
            self.assertRaisesRegex(ValueError, "manifest digest mismatch"),
        ):
            self.qualify({})


if __name__ == "__main__":
    unittest.main()
