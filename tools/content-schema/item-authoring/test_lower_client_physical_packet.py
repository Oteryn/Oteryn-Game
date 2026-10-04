"""Fail-closed field qualification and source-binding evidence tests."""

import unittest
from pathlib import Path

import lower_client_physical_packet as lower


class PhysicalPacketTests(unittest.TestCase):
    def test_whole_retained_scope_is_exact_bound_with_affirmative_object_proof(self):
        packet = lower.build()
        self.assertEqual(packet["counts"]["fields"], 6833)
        self.assertEqual(packet["counts"]["distinct_items"], 6755)
        self.assertEqual(packet["counts"]["RUNE_STACK_100"], 39)
        for row in packet["promotions"]:
            source = row["source"]
            self.assertEqual(source["binding"]["target"]["key"], row["item_key"])
            self.assertEqual(
                int(source["binding"]["external_id"]), source["appearance_id"]
            )
            self.assertEqual(len(source["object_sha256"]), 64)
        self.assertEqual(
            sum(r["reason"] == "EXISTING_MAP_OWNER" for r in packet["holds"]), 241
        )
        # D289: the accepted Native core hold keeps i901 out of the promotions.
        self.assertEqual(
            [
                r["item_key"]
                for r in packet["holds"]
                if r["reason"] == "D289_ACCEPTED_NATIVE_CORE_HOLD"
            ],
            ["oteryn:item.tibia.i901"],
        )
        self.assertNotIn(
            "oteryn:item.tibia.i901", {r["item_key"] for r in packet["promotions"]}
        )

    def test_blocked_or_conflicting_field_never_becomes_known(self):
        for field in [
            {"state": "CONFLICT"},
            {"state": "NOT_APPLICABLE"},
            {"state": "KNOWN", "value": False},
        ]:
            with self.assertRaises(ValueError):
                lower.leaf(
                    {"physical": {"state": "KNOWN", "value": {"pickupable": field}}},
                    "physical",
                    "pickupable",
                    True,
                )
        lower.leaf({}, "physical", "pickupable", True)
        lower.leaf(
            {
                "physical": {
                    "state": "KNOWN",
                    "value": {"pickupable": {"state": "KNOWN", "value": True}},
                }
            },
            "physical",
            "pickupable",
            True,
        )
        with self.assertRaises(ValueError):
            lower.leaf(
                {"physical": {"state": "CONFLICT"}}, "physical", "pickupable", True
            )

    def test_pinned_source_drift_is_rejected(self):
        with self.assertRaises(ValueError):
            lower.checked(Path(__file__).parent, Path(__file__).name, "0" * 64)


if __name__ == "__main__":
    unittest.main()
