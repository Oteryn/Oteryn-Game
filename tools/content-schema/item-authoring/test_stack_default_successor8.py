"""Closed successor source and repository-relative full replay guards."""

import ast
import copy
import json
import unittest
from pathlib import Path
from unittest.mock import patch

import lower_wiki_stack_default_successor8_packet as target


class Successor8SourceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.proof = json.loads(
            target.base.checked(target.ROOT, target.PROOF, target.PROOF_SHA)
        )
        cls.frame = json.loads(
            target.base.checked(target.ROOT, target.RECEIPT, target.RECEIPT_SHA)
        )["own_id_source_frame"]
        cls.inputs = target.context.current_inputs(target.ROOT)

    def qualify(self, row, **changes):
        _, objects, bound, wiki, pages, definitions, routed = self.inputs
        iid, key = row["source"]["source_item_id"], row["source"]["item_key"]
        values = {
            "definition": definitions[key],
            "binding": bound[key],
            "obj": objects.get(iid, {}),
            "routed": routed,
            "observations": wiki.get(iid, []),
            "pages": pages,
            "cutoff": self.proof["qualification_cutoff"],
        }
        values.update(changes)
        return target.qualify(row, **values)

    def test_full_global_index_and_actual_part_replayed(self):
        frame = copy.deepcopy(self.frame)
        target.own_id_frame(target.ROOT, self.proof, frame)

    def test_own_reference_and_raw_box_metadata_cannot_replace_artifacts(self):
        frame = copy.deepcopy(self.frame)
        for field in ("complete_selected_own_id_references", "selected_own_raw_box"):
            proof = copy.deepcopy(self.proof)
            if field == "complete_selected_own_id_references":
                proof["records"][0][field][0]["page_ordinal"] += 1
            else:
                proof["records"][0][field]["raw"] += "forged"
            with self.assertRaises(ValueError):
                target.own_id_frame(target.ROOT, proof, frame)
        for path_target in ("index", "part"):
            substituted = copy.deepcopy(self.frame)
            pin = (
                substituted["index"]
                if path_target == "index"
                else substituted["parts"][0]
            )
            pin["path"] = "/workspace/audit-continuation/substituted.json"
            with self.assertRaises(ValueError):
                target.own_id_frame(target.ROOT, self.proof, substituted)
        frame["parts"][0]["sha256"] = "0" * 64
        with self.assertRaises(ValueError):
            target.own_id_frame(target.ROOT, self.proof, frame)

    def test_exact_comparator_does_not_equate_booleans_and_numbers(self):
        source = Path(__file__).with_name("verify_default8_exact_delta.py")
        parsed = ast.parse(source.read_bytes())
        functions = [
            node
            for node in parsed.body
            if isinstance(node, ast.FunctionDef)
            and node.name in {"canonical", "false_census"}
        ]
        namespace = {"json": json}
        exec(  # noqa: S102 - Only two inspected local pure functions, no CLI or imports.
            compile(ast.Module(body=functions, type_ignores=[]), str(source), "exec"),
            namespace,
        )
        canonical, census = namespace["canonical"], namespace["false_census"]
        self.assertNotEqual(canonical({"value": False}), canonical({"value": 0}))
        self.assertNotEqual(canonical({"value": True}), canonical({"value": 1}))
        rows = [
            {
                "semantics": {
                    "stack": {
                        "value": {"stackable": {"state": "KNOWN", "value": value}}
                    }
                }
            }
            for value in (False, 0, True, 1)
        ]
        self.assertEqual(census(rows), 1)

    def test_closed_eight_and_false_is_derived(self):
        self.assertEqual(
            {r["source"]["source_item_id"] for r in self.proof["records"]}, target.IDS
        )
        for row in self.proof["records"]:
            self.assertFalse(self.qualify(row)["stackable"])
            self.assertEqual(row["evidence"]["client_cumulative"]["state"], "UNKNOWN")

    def test_source_identity_and_late_domain_conflicts_rejected(self):
        row = self.proof["records"][0]
        for field in ("content_sha256", "source_item_id"):
            changed = copy.deepcopy(row)
            changed["source"][field] = "foreign"
            with self.assertRaises(ValueError):
                self.qualify(changed)
        with self.assertRaises(ValueError):
            self.qualify(row, routed={row["source"]["item_key"]})
        changed = copy.deepcopy(row["binding"])
        changed["target"]["revision"] = "foreign"
        with self.assertRaises(ValueError):
            self.qualify(row, binding=changed)

    def test_present_empty_stackable_is_not_a_default(self):
        row = copy.deepcopy(self.proof["records"][0])
        box = row["selected_own_raw_box"]["raw"]
        amended = box[:-2] + "|stackable=\n}}"
        row["source"]["content"] = row["source"]["content"].replace(box, amended)
        row["source"]["content_sha256"] = target.base.sha(
            row["source"]["content"].encode()
        )
        with self.assertRaises(ValueError):
            self.qualify(row)

    def test_live_siblings_are_independent_but_identity_name_headers_stay_guarded(self):
        row = self.proof["records"][0]
        _, _, _, _, _, definitions, _ = self.inputs
        definition = copy.deepcopy(definitions[row["source"]["item_key"]])
        definition["semantics"]["presentation"]["value"]["description"] = {
            "state": "KNOWN",
            "value": "Later independently qualified description",
        }
        definition["semantics"]["weapon"]["value"]["defense"] = {
            "state": "KNOWN",
            "value": 17,
        }
        definition["semantics"]["physical"]["value"]["weight"] = {
            "state": "KNOWN",
            "value": 900,
        }
        self.assertFalse(self.qualify(row, definition=definition)["stackable"])
        updated = copy.deepcopy(definitions)
        updated[row["source"]["item_key"]] = definition
        outside = next(
            key
            for key, d in updated.items()
            if key not in {r["source"]["item_key"] for r in self.proof["records"]}
            and d.get("semantics", {}).get("presentation", {}).get("state") == "KNOWN"
        )
        updated[outside]["semantics"]["presentation"]["value"]["description"] = {
            "state": "KNOWN",
            "value": "Independent outside description",
        }
        current = (*self.inputs[:5], updated, self.inputs[6])
        with patch.object(target.context, "current_inputs", return_value=current):
            self.assertEqual(
                target.context.canonical(target.build()),
                (target.ROOT / target.OUTPUT).read_bytes(),
            )
        for field, value in (
            ("kind", "Creature"),
            ("stack_class", "StackCapable"),
            ("materializable", True),
            ("materializable", 0),
            ("client_projection", "ServerOnly"),
        ):
            changed = copy.deepcopy(definition)
            changed[field] = value
            with self.assertRaises(ValueError):
                self.qualify(row, definition=changed)
        for value in (
            {"state": "UNKNOWN"},
            {"state": "KNOWN", "value": "foreign name"},
        ):
            changed = copy.deepcopy(definition)
            changed["semantics"]["presentation"]["value"]["name"] = value
            with self.assertRaises(ValueError):
                self.qualify(row, definition=changed)

    def test_current_native_states_and_idempotence(self):
        row = self.proof["records"][0]
        for state in (
            {"state": "CONFLICT"},
            {"state": "NOT_APPLICABLE"},
            {
                "state": "KNOWN",
                "value": {"stackable": {"state": "KNOWN", "value": True}},
            },
        ):
            definition = copy.deepcopy(row["current_native_definition"])
            definition["semantics"]["stack"] = state
            with self.assertRaises(ValueError):
                self.qualify(row, definition=definition)
        definition = copy.deepcopy(row["current_native_definition"])
        definition["semantics"]["stack"] = {
            "state": "KNOWN",
            "value": {
                "stackable": {"state": "KNOWN", "value": False},
                "stack_max": {"state": "KNOWN", "value": 7},
            },
        }
        self.assertFalse(self.qualify(row, definition=definition)["stackable"])


if __name__ == "__main__":
    unittest.main()
