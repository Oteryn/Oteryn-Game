"""Amber source qualification, exact repair scope and conflict preservation."""
import copy
import json
import unittest
from unittest.mock import patch

import amber_item_semantics as tool


class AmberItemSemantics(unittest.TestCase):
    def setUp(self):
        shard = json.loads((tool.ROOT / "content/items/definitions/items-27500-27999.json").read_bytes())
        self.definition = copy.deepcopy(next(r["definition"] for r in shard["records"]
                                             if r["definition"]["identity"]["key"] == tool.KEY))

    def test_source_qualified_repair_changes_only_container_and_is_idempotent(self):
        self.definition["semantics"]["container"] = {"state": "UNKNOWN"}
        expected = copy.deepcopy(self.definition)
        expected["semantics"]["container"] = {"state": "NOT_APPLICABLE"}
        tool.apply_container_fact([self.definition])
        self.assertEqual(self.definition, expected)
        tool.apply_container_fact([self.definition])
        self.assertEqual(self.definition, expected)
        patterns = self.definition["semantics"]["equipment"]["value"]["patterns"]["value"]
        self.assertEqual(patterns[0]["primary_slot"], {"state": "KNOWN", "value": "WEAPON"})
        self.assertEqual(patterns[0]["additional_reserved_slots"], {"state": "KNOWN", "value": ["SHIELD"]})

    def test_known_container_conflict_is_preserved(self):
        self.definition["semantics"]["container"] = {"state": "KNOWN", "value": {"capacity": {"state": "KNOWN", "value": 8}}}
        before = copy.deepcopy(self.definition)
        with self.assertRaisesRegex(ValueError, "AMBER_CONTAINER_CONFLICT"):
            tool.apply_container_fact([self.definition])
        self.assertEqual(self.definition, before)

    def test_source_drift_does_not_modify_item(self):
        before = copy.deepcopy(self.definition)
        with patch.object(tool, "XML_SHA", "0" * 64):
            with self.assertRaisesRegex(ValueError, "AMBER_SOURCE_DRIFT"):
                tool.apply_container_fact([self.definition])
        self.assertEqual(self.definition, before)


if __name__ == "__main__":
    unittest.main()
