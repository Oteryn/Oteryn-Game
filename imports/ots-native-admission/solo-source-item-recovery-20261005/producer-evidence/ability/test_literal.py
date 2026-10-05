"""Source declaration boundaries and exact bounded input framing."""

import copy
import json
import tempfile
import unittest
from pathlib import Path

import jsonschema
from literal import emit_literal
from prepare_ledger import blocks, prepare
from schema import schema


def assignment(key, value, ordinal=0):
    return {
        "attribute_ordinal": ordinal,
        "key": key,
        "key_lexeme": key,
        "value_lexeme": value,
        "decoded_value": value,
        "captured_attribute_projection_sha256": "0" * 64,
    }


class LiteralTests(unittest.TestCase):
    def test_overflow_and_error_lexemes_remain_source_inputs_not_fake_results(self):
        validator = jsonschema.Draft202012Validator(schema())
        for value in [
            "2147483648",
            "-9999999999999999999999",
            "1tail",
            "+1",
            " 1",
            "not-number",
        ]:
            inputs = [assignment("speed", value)]
            trace = emit_literal("canary-47df", inputs)
            validator.validate(trace)
            self.assertEqual(trace["ordered_assignments"], inputs)
            event = next(
                e for e in trace["ordered_events"] if e["handler"] == "parseSpeed"
            )
            self.assertEqual(event["numeric_result"], {"state": "UNKNOWN"})
            self.assertEqual(trace["actual_stored_allocation"], {"state": "UNKNOWN"})
            self.assertEqual(trace["final_sparse_members"], {"state": "UNKNOWN"})
        bad = copy.deepcopy(trace)
        bad["final_sparse_members"] = {"state": "KNOWN", "value": []}
        self.assertFalse(validator.is_valid(bad))
        bad = copy.deepcopy(trace)
        bad["ordered_events"][0]["kind"] = "NEW_ENGINE_KIND"
        self.assertFalse(validator.is_valid(bad))

    def test_truthy_NONE_false_sticky_alias_and_cut_order_are_retained(self):
        for cut in ["canary-47df", "crystal-ff7", "crystal-00ce"]:
            yes = emit_literal(cut, [assignment("armor", "1")])
            no = emit_literal(cut, [assignment("armor", "0")])
            self.assertEqual(
                yes["declared_first_getter"]["handler"], "parseSupressDrunk"
            )
            self.assertTrue(yes["ordered_events"][0]["suppression_default_NONE"])
            self.assertEqual(
                no["declared_first_getter"]["handler"],
                "parseSpecializedMagicLevelPoint",
            )
            ordered = emit_literal(
                cut,
                [assignment("suppressfire", "1"), assignment("suppressfire", "0", 1)],
            )
            self.assertEqual(
                [e["branch"] for e in ordered["ordered_events"]],
                ["DECLARED_CONDITIONAL_SETTER", "FALSE_NO_WRITE"],
            )
            alias = emit_literal(cut, [assignment("magicpointspercent", "10")])
            self.assertEqual(
                alias["ordered_events"][-1]["branch"], "DISPATCH_ALIAS_NO_SETTER"
            )
            unknown = emit_literal(cut, [assignment("magiclevelpointspercent", "1")])
            self.assertIsNone(unknown["declared_first_getter"])
            self.assertEqual(
                unknown["ordered_assignments"][0]["key"], "magiclevelpointspercent"
            )
        inputs = [assignment("mantra", "bad")]
        self.assertFalse(
            any(
                e["kind"].startswith("CRYSTAL_MANTRA")
                for e in emit_literal("canary-47df", inputs)["ordered_events"]
            )
        )
        self.assertTrue(
            any(
                e["kind"].startswith("CRYSTAL_MANTRA")
                for e in emit_literal("crystal-ff7", inputs)["ordered_events"]
            )
        )

    def test_exact_frozen_pretty_blocks_and_preopen_resource_guard(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "fixture.json"
            value = {
                "schema": "fixture",
                "targets": {
                    "oteryn:item1": {"r": [{"x": 1}]},
                    "oteryn:item2": {"r": [{"x": 2}]},
                },
            }
            path.write_text(json.dumps(value, indent=2) + "\n")
            self.assertEqual(
                list(blocks(path, b'  "targets": {', b'    "oteryn:item', b"    }")),
                [
                    {"oteryn:item1": value["targets"]["oteryn:item1"]},
                    {"oteryn:item2": value["targets"]["oteryn:item2"]},
                ],
            )
            path.write_text(
                json.dumps(
                    {"schema": "fixture", "records": [{"x": 1}, {"x": 2}]}, indent=2
                )
                + "\n"
            )
            self.assertEqual(
                list(blocks(path, b'  "records": [', b"    {", b"    }")),
                [{"x": 1}, {"x": 2}],
            )
        with self.assertRaisesRegex(ValueError, "exclusive"):
            prepare(Path("/never/read"), Path("/never/write"))


if __name__ == "__main__":
    unittest.main()
