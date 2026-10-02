"""Prepared source guards; external fixture path is replaced upon allocation."""

import copy
import json
import unittest
from pathlib import Path

import lower_wiki_stack_default_successor8_packet as target


class Successor8SourceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.proof = json.loads(
            Path(
                "/workspace/audit-continuation/"
                "stack-default-successor8-source-qualification.json"
            ).read_bytes()
        )
        cls.inputs = target.context.current_inputs(target.ROOT)

    def qualify(self, row, **changes):
        _, objects, bound, wiki, pages, definitions, routed = self.inputs
        iid, key = row["source"]["source_item_id"], row["source"]["item_key"]
        values = dict(
            definition=definitions[key],
            binding=bound[key],
            obj=objects.get(iid, {}),
            routed=routed,
            observations=wiki.get(iid, []),
            pages=pages,
            cutoff=self.proof["qualification_cutoff"],
        )
        values.update(changes)
        return target.qualify(row, **values)

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
