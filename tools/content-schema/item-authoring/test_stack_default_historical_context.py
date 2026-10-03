"""Historical evidence reproduction never bypasses current source/Native guards."""

import copy
import json
import tempfile
import unittest
from pathlib import Path

import check_stack_default_historical_context as check


class ContextTests(unittest.TestCase):
    def test_packets_are_byte_exact_and_actual_current_cohort_stays_closed(self):
        result = check.check()
        self.assertEqual(result["strict_current_original_defaults"], 1487)
        self.assertEqual(result["strict_current_original_historical"], 7)
        self.assertEqual(result["outside_cohort_promotions_written"], 0)

    def test_frozen_context_byte_substitution_is_rejected(self):
        with tempfile.TemporaryDirectory(
            prefix="negative-frozen-context-"
        ) as directory:
            root = Path(directory)
            path = root / check.CONTEXT
            path.parent.mkdir(parents=True)
            path.write_bytes((check.ROOT / check.CONTEXT).read_bytes() + b" ")
            with self.assertRaisesRegex(ValueError, "source digest drift"):
                check.read_context(root)

    def test_original_proof_source_bytes_cannot_be_substituted(self):
        with tempfile.TemporaryDirectory(prefix="negative-source-bytes-") as directory:
            root = Path(directory)
            for path in (check.CONTEXT, check.base.PROOF):
                target = root / path
                target.parent.mkdir(parents=True, exist_ok=True)
                data = (check.ROOT / path).read_bytes()
                target.write_bytes(data + (b" " if path == check.base.PROOF else b""))
            with self.assertRaisesRegex(ValueError, "source digest drift"):
                check.read_context(root)

    def test_actual_current_name_conflict_rejects_the_original_cohort(self):
        source, _, _, _, _, current, _ = check.current_inputs(check.ROOT)
        packet = json.loads(check.base.OUTPUT.read_text())
        promoted = packet["promotions"][0]["item_key"]
        # Negative-only isolated fixture starts with actual current source-scope definitions.
        definitions = [copy.deepcopy(current[s["item_key"]]) for s in source["records"]]
        victim = next(d for d in definitions if d["identity"]["key"] == promoted)
        victim["semantics"]["presentation"] = {
            "state": "KNOWN",
            "value": {
                "name": {
                    "state": "KNOWN",
                    "value": "deliberately opposing current name",
                }
            },
        }
        with tempfile.TemporaryDirectory(
            prefix="negative-current-name-fixture-"
        ) as directory:
            negative_root = Path(directory)
            for name in ("docs", "tools", "imports"):
                (negative_root / name).symlink_to(
                    check.ROOT / name, target_is_directory=True
                )
            (negative_root / "content/items").mkdir(parents=True)
            for name in ("assets", "world"):
                (negative_root / "content" / name).symlink_to(
                    check.ROOT / "content" / name, target_is_directory=True
                )
            path = "content/items/negative-current-fixture-definitions.json"
            (negative_root / "content/items/index.json").write_bytes(
                check.canonical(
                    {"purpose": "NEGATIVE_TEST_ONLY_NOT_AUTHORITY", "shards": [path]}
                )
            )
            shard = negative_root / path
            shard.write_bytes(
                check.canonical({"records": [{"definition": d} for d in definitions]})
            )
            with self.assertRaisesRegex(ValueError, "KNOWN_PRESENTATION_NAME_CONFLICT"):
                check.current_validation(negative_root)
        self.assertNotEqual(
            check.current_inputs(check.ROOT)[5][promoted]["semantics"].get(
                "presentation"
            ),
            victim["semantics"]["presentation"],
        )


if __name__ == "__main__":
    unittest.main()
