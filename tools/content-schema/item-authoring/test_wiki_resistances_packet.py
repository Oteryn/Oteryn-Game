"""Closed, exact source resistance vectors; variant and known evidence stay fenced."""

import copy
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import lower_wiki_stats_packet as lower


class ResistanceTests(unittest.TestCase):
    key = "oteryn:item.tibia.i1"

    def qualify(
        self,
        values=("fire +5%, physical -2%",),
        *,
        definition=None,
        routed=(),
        shared=False,
        ids=(1,),
    ):
        observations = [
            {
                "page_id": index + 10,
                "revision_id": 20,
                "fields": {} if raw is None else {"resist": raw},
            }
            for index, raw in enumerate(values)
        ]
        records = {self.key: {"item_id": 1, "observations": observations}}
        if shared:
            records["unbound"] = {
                "item_id": 999,
                "observations": [{"page_id": 10, "revision_id": 20, "fields": {}}],
            }
        if definition is None:
            definition = {"semantics": {"protection": {"state": "UNKNOWN"}}}
        rows, report, _ = lower.build(
            {"records": records}, set(ids), {self.key: definition}, set(routed)
        )
        return rows, report["physical_field_holds"]

    def test_external_conflict_and_manifest_or_snapshot_drift_fail_closed(self):
        hold = lower.source_hold()
        key = hold["item_key"]
        expected = hold["expected_snapshot_sources"][0]
        obs = {name: value for name, value in expected.items() if name != "values"}
        obs["fields"] = expected["values"] | {"armor": "8"}
        snapshot = {"records": {key: {"item_id": 50275, "observations": [obs]}}}
        defs = {key: {"semantics": {"protection": {"state": "UNKNOWN"}}}}
        rows, report, _ = lower.build(snapshot, {50275}, defs)
        self.assertEqual([row["field_path"] for row in rows], ["protection.armor"])
        blocked = report["physical_field_holds"][0]
        self.assertEqual(
            (blocked["classification"], blocked["reason"]),
            ("CONFLICT", "EXTERNAL_SOURCE_DISAGREEMENT"),
        )
        self.assertEqual(blocked["external_qualification"], hold)
        for field, value in (
            ("page_id", 0),
            ("revision_id", 0),
            ("content_sha256", "drift"),
            ("fields", {}),
        ):
            changed = copy.deepcopy(snapshot)
            changed["records"][key]["observations"][0][field] = value
            with self.assertRaisesRegex(ValueError, "snapshot facts drift"):
                lower.build(changed, {50275}, defs)
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "hold.json"
            path.write_bytes(lower.SOURCE_HOLD.read_bytes() + b" ")
            with (
                patch.object(lower, "SOURCE_HOLD", path),
                self.assertRaisesRegex(ValueError, "manifest digest mismatch"),
            ):
                self.qualify()

    def test_percentage_points_sign_zero_precision_and_native_order(self):
        rows, holds = self.qualify(
            ("physical +4%, fire -8%, drowning +100%, earth +0.125%, ice -0%",)
        )
        self.assertEqual(holds, [])
        _, bound = lower.resistances({"resist": "earth -100%"})
        self.assertEqual(
            bound["value"][0]["percent"], {"numerator": -100, "denominator": 1}
        )
        self.assertEqual(
            rows[0]["typed_value"],
            {
                "kind": "RESISTANCES",
                "value": [
                    {"kind": "DROWN", "percent": {"numerator": 100, "denominator": 1}},
                    {"kind": "EARTH", "percent": {"numerator": 1, "denominator": 8}},
                    {"kind": "FIRE", "percent": {"numerator": -8, "denominator": 1}},
                    {"kind": "ICE", "percent": {"numerator": 0, "denominator": 1}},
                    {"kind": "PHYSICAL", "percent": {"numerator": 4, "denominator": 1}},
                ],
            },
        )
        _, typed = lower.resistances(
            {"resist": "poison +5%, fire field -5%, life drain 2%, mana drain +3%"}
        )
        self.assertEqual(
            [row["kind"] for row in typed["value"]],
            ["LIFE_DRAIN", "MANA_DRAIN", "POISON", "FIRE_FIELD"],
        )

    def test_malformed_unsupported_duplicate_or_outside_bounds_holds_whole_vector(self):
        for value in (
            "",
            "fire +5%,",
            "fire +5%, physical unknown",
            "critical hit chance +5%",
            "fire 1/2%",
            "fire +5%, fire +5%",
            "drown +1%, drowning +2%",
            "fire +100.1%",
            "ice -101%",
            "physical +0.00000000000000000001%",
            "fire +9223372036854775808%",
        ):
            with self.subTest(value=value):
                rows, holds = self.qualify((value,))
                self.assertEqual(rows, [])
                self.assertEqual(holds[0]["reason"], "MALFORMED_WIKI_VALUE")
        self.assertEqual(self.qualify((None,)), ([], []))

    def test_page_agreement_is_exact_whole_vector_and_no_majority(self):
        rows, holds = self.qualify(("fire +5%, earth +8%", "earth 8.0%, fire 5%", None))
        self.assertEqual(len(rows), 1)
        self.assertEqual(holds, [])
        rows, holds = self.qualify(("fire +5%", "fire +5%", "fire +8%"))
        self.assertEqual(rows, [])
        self.assertEqual(holds[0]["reason"], "WIKI_PAGE_DISAGREEMENT")
        self.assertEqual(len(holds[0]["sources"]), 3)

    def test_shared_unbound_source_id_map_owner_and_no_canonical_semantics(self):
        rows, holds = self.qualify(shared=True)
        self.assertEqual(rows, [])
        self.assertEqual(holds[0]["reason"], "SHARED_PAGE_VARIANT_UNQUALIFIED")
        self.assertEqual(holds[0]["sources"][0]["page_item_ids"], [1, 999])
        self.assertEqual(self.qualify(ids=()), ([], []))
        for args, reason in (
            ({"routed": (self.key,)}, "EXISTING_MAP_OWNER"),
            ({"definition": {}}, "NO_CANONICAL_ITEM_SEMANTICS"),
        ):
            rows, holds = self.qualify(**args)
            self.assertEqual(rows, [])
            self.assertEqual(holds[0]["reason"], reason)

    def test_known_vector_idempotence_conflict_and_blocked_group_leaf_or_percent(self):
        values = [
            {
                "kind": "FIRE",
                "percent": lower.known({"numerator": 5, "denominator": 1}),
            },
            {
                "kind": "PHYSICAL",
                "percent": lower.known({"numerator": -2, "denominator": 1}),
            },
        ]
        definition = {
            "semantics": {
                "protection": lower.known({"resistances": lower.known(values)})
            }
        }
        self.assertEqual(self.qualify(definition=definition), self.qualify())
        bad = copy.deepcopy(definition)
        bad["semantics"]["protection"]["value"]["resistances"]["value"][0]["percent"][
            "value"
        ]["numerator"] = 6
        self.assertEqual(
            self.qualify(definition=bad)[1][0]["reason"], "KNOWN_FIELD_CONFLICT"
        )
        for state in ("CONFLICT", "NOT_APPLICABLE"):
            for where in ("group", "leaf", "percent"):
                bad = copy.deepcopy(definition)
                if where == "group":
                    bad["semantics"]["protection"] = {"state": state}
                elif where == "leaf":
                    bad["semantics"]["protection"]["value"]["resistances"] = {
                        "state": state
                    }
                else:
                    bad["semantics"]["protection"]["value"]["resistances"]["value"][0][
                        "percent"
                    ] = {"state": state}
                self.assertEqual(
                    self.qualify(definition=bad)[1][0]["reason"],
                    "BLOCKED_EVIDENCE_STATE",
                )
        bad = copy.deepcopy(definition)
        bad["semantics"]["protection"]["value"]["resistances"]["value"][0][
            "percent"
        ] = {"state": "UNKNOWN"}
        self.assertEqual(
            self.qualify(definition=bad)[1][0]["reason"],
            "KNOWN_PARTIAL_VECTOR_UNQUALIFIED",
        )


if __name__ == "__main__":
    unittest.main()
