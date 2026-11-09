import json
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator


class CounterFloorSchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        source = json.loads(Path(__file__).with_name("shared-reward-callbacks.schema.json").read_text())
        cls.validator = Draft202012Validator({"$defs": source["$defs"], "$ref": "#/$defs/stateOperation"})

    def operation(self):
        return {"track": "oteryn:quest-progress/m07/counter", "from": {"kind": "any"},
                "effect": {"kind": "add_at_least", "value": 1, "minimum": 2}}

    def test_source_counter_floor_is_authorable(self):
        self.assertEqual(list(self.validator.iter_errors(self.operation())), [])

    def test_floor_requires_exact_signed_integer_terms(self):
        for field in ("value", "minimum"):
            for value in (None, "2", 2.5, -(2**63)-1, 2**63):
                operation = self.operation()
                operation["effect"][field] = value
                self.assertTrue(list(self.validator.iter_errors(operation)), (field, value))
            operation = self.operation()
            del operation["effect"][field]
            self.assertTrue(list(self.validator.iter_errors(operation)))

    def test_legacy_add_and_transaction_clock_remain_authorable(self):
        for effect in ({"kind": "add", "value": 1}, {"kind": "set_now"}):
            operation = self.operation()
            operation["effect"] = effect
            self.assertEqual(list(self.validator.iter_errors(operation)), [])


if __name__ == "__main__":
    unittest.main()
