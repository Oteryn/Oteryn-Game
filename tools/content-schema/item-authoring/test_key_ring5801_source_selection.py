"""Exact-source selection, drift, and native-state guards; no network calls."""

import copy
import json
import tempfile
import unittest
from pathlib import Path

import key_ring5801_source_selection as selection

ROOT = Path(__file__).resolve().parents[3]
MANIFEST = json.loads((ROOT / selection.MANIFEST).read_text())


class Qualification(unittest.TestCase):
    def inputs(self):
        return [
            copy.deepcopy(MANIFEST),
            {
                "item_id": 5801,
                "observations": copy.deepcopy(
                    MANIFEST["expected_snapshot_observations"]
                ),
            },
            {
                "identity": {"key": selection.KEY},
                "semantics": {
                    "container": {
                        "state": "KNOWN",
                        "value": {"capacity": {"state": "KNOWN", "value": 22}},
                    },
                    "physical": {"state": "UNKNOWN"},
                    "presentation": {"state": "UNKNOWN"},
                },
            },
            copy.deepcopy(MANIFEST["exact_binding"]),
            {
                "id": 5801,
                "name": "jewelled backpack",
                "flags": {
                    "flags.container": True,
                    "flags.take": True,
                    "market.category": 4,
                    "market.trade_as_object_id": 5801,
                    "market.show_as_object_id": 5801,
                },
            },
        ]

    def test_current_values_with_full_retired_evidence_retained(self):
        args = self.inputs()
        before = copy.deepcopy(args)
        context = selection.qualify(*args)
        for field in selection.SCOPE:
            rows = selection.select(args[1], field, context)
            self.assertEqual(len(rows), 1)
            self.assertEqual(rows[0]["fields"]["weight"], "17.00")
            self.assertEqual(rows[0]["fields"]["volume"], "22")
        self.assertEqual(context["qualification"]["classification"], "DERIVED")
        self.assertEqual(
            context["qualification"]["excluded_historical_observation"][
                "historical_values"
            ]["weight"],
            "0.50",
        )
        self.assertEqual(args, before)
        args[2]["semantics"]["physical"] = {
            "state": "KNOWN",
            "value": {"weight": {"state": "KNOWN", "value": 1700}},
        }
        self.assertEqual(selection.qualify(*args), context)

    def test_modified_manifest_is_rejected_before_other_input_loading(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / selection.MANIFEST
            path.parent.mkdir(parents=True)
            path.write_text("{}")
            with self.assertRaisesRegex(ValueError, "manifest digest"):
                selection.load_context(root, {}, {}, set())

    def test_scope_keeps_unselected_fields_and_ids_unchanged(self):
        args = self.inputs()
        context = selection.qualify(*args)
        for field, iid in [
            ("equipment.patterns", 5801),
            ("physical.weight", 1),
            ("charges.count", 5801),
        ]:
            record = args[1] | {"item_id": iid}
            self.assertIs(
                selection.select(record, field, context), record["observations"]
            )
        self.assertIs(
            selection.select(args[1], "physical.weight"), args[1]["observations"]
        )

    def test_exact_inputs_and_positive_client_guards(self):
        mutations = [
            lambda a: a[3].update(disposition="NAME_MATCH"),
            lambda a: a[3].update(identity_namespace="official/item_id"),
            lambda a: a[3]["target"].update(key="other"),
            lambda a: a[1]["observations"][0].update(revision_id=1012665),
            lambda a: a[1]["observations"].append(
                copy.deepcopy(a[1]["observations"][0])
            ),
            lambda a: a[1]["observations"][1]["fields"].update(volume="23"),
            lambda a: a[0]["retained_raw_wiki_observations"][0].update(
                wikitext="changed"
            ),
            lambda a: a[0]["official_retirement"].update(quote="different"),
            lambda a: a[4].update(name="key ring"),
            lambda a: a[4]["flags"].pop("flags.container"),
            lambda a: a[4]["flags"].update({"flags.take": False}),
            lambda a: a[4]["flags"].update({"market.trade_as_object_id": 1}),
        ]
        for mutate in mutations:
            with self.subTest(mutation=mutate):
                args = self.inputs()
                mutate(args)
                with self.assertRaises(ValueError):
                    selection.qualify(*args)
        with self.assertRaises(ValueError):
            selection.qualify(*self.inputs(), routed=True)

    def test_blocked_groups_fields_and_known_differences_hold(self):
        for group, field, bad_value in [
            ("physical", "weight", 50),
            ("container", "capacity", 20),
        ]:
            for state in ["CONFLICT", "NOT_APPLICABLE", "BLOCKED"]:
                for leaf in [False, True]:
                    args = self.inputs()
                    envelope = {"state": state}
                    if leaf:
                        envelope = {"state": "KNOWN", "value": {field: envelope}}
                    args[2]["semantics"][group] = envelope
                    with (
                        self.subTest(group=group, state=state, leaf=leaf),
                        self.assertRaises(ValueError),
                    ):
                        selection.qualify(*args)
            args = self.inputs()
            args[2]["semantics"][group] = {
                "state": "KNOWN",
                "value": {field: {"state": "KNOWN", "value": bad_value}},
            }
            with self.assertRaises(ValueError):
                selection.qualify(*args)
        args = self.inputs()
        args[2]["semantics"]["presentation"] = {"state": "CONFLICT"}
        with self.assertRaises(ValueError):
            selection.qualify(*args)


if __name__ == "__main__":
    unittest.main()
