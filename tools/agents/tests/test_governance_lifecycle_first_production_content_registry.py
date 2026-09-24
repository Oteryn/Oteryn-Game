from __future__ import annotations

import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[3]
REGISTRY = ROOT / "docs/contracts/RESOURCE_LIMITS_REGISTRY.json"
ARCH = ROOT / "docs/architecture"
BASE = ARCH / "OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09.md"
AMENDMENT_01 = ARCH / "OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09_AMENDMENT_01.md"
AMENDMENT_02 = ARCH / "OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09_AMENDMENT_02.md"
AMENDMENT_03 = ARCH / "OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09_AMENDMENT_03.md"
PREFIX = "DUR04-FIRST-PROD-"


def json_block_after(text: str, marker: str) -> list[dict[str, object]]:
    start = text.index(marker)
    fence = text.index("```json", start) + len("```json")
    end = text.index("```", fence)
    value = json.loads(text[fence:end].strip())
    if not isinstance(value, list) or not all(isinstance(row, dict) for row in value):
        raise AssertionError(f"expected JSON object array after {marker}")
    return value


def expected_first_production_rows() -> list[dict[str, object]]:
    base_text = BASE.read_text(encoding="utf-8")
    a1_text = AMENDMENT_01.read_text(encoding="utf-8")
    a2_text = AMENDMENT_02.read_text(encoding="utf-8")
    a3_text = AMENDMENT_03.read_text(encoding="utf-8")

    rows = [dict(row) for row in json_block_after(
        base_text, "## 6. Exact `RESOURCE_LIMITS_REGISTRY.json` serialized append"
    )]
    a1_rows = json_block_after(
        a1_text, "## 3. Exact serialized `RESOURCE_LIMITS_REGISTRY.json` additions"
    )
    a2_new = json_block_after(
        a2_text, "### 5.1 New package/provenance registry objects"
    )
    a2_replacements = json_block_after(
        a2_text, "### 5.2 Exact replacements for base registry objects"
    )
    a3_replacements = json_block_after(
        a3_text, "## 3. Exact `RESOURCE_LIMITS_REGISTRY.json` serialization correction"
    )

    def insert_after(after_id: str, new_rows: list[dict[str, object]]) -> None:
        index = next(i for i, row in enumerate(rows) if row["id"] == after_id) + 1
        rows[index:index] = [dict(row) for row in new_rows]

    def replace_by_id(replacements: list[dict[str, object]]) -> None:
        positions = {str(row["id"]): i for i, row in enumerate(rows)}
        for replacement in replacements:
            replacement_id = str(replacement["id"])
            if replacement_id not in positions:
                raise AssertionError(f"replacement target missing: {replacement_id}")
            rows[positions[replacement_id]] = dict(replacement)

    insert_after("DUR04-FIRST-PROD-PACKAGES", a2_new)
    insert_after("DUR04-FIRST-PROD-SPAWNS", a1_rows)
    replace_by_id(a2_replacements)
    replace_by_id(a3_replacements)
    return rows


class FirstProductionContentRegistryContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.registry_text = REGISTRY.read_text(encoding="utf-8")
        cls.registry = json.loads(cls.registry_text)
        cls.expected = expected_first_production_rows()
        cls.actual = [
            row for row in cls.registry["entries"]
            if isinstance(row, dict) and str(row.get("id", "")).startswith(PREFIX)
        ]

    def test_exact_serialization_matches_protected_decision_packet(self) -> None:
        self.assertEqual(46, len(self.expected))
        self.assertEqual(46, len(self.actual))
        self.assertEqual(self.expected, self.actual)

    def test_appended_rows_have_complete_bounded_contract_shape(self) -> None:
        required = set(self.registry["required_entry_fields"])
        all_ids = [row["id"] for row in self.registry["entries"]]
        self.assertEqual(len(all_ids), len(set(all_ids)), "registry IDs must be globally unique")

        ambiguous_units = {"", "unit", "units", "value", "values", "amount", "size", "count"}
        for row in self.actual:
            row_id = str(row["id"])
            self.assertTrue(required.issubset(row), f"{row_id}: missing required field")
            maximum = row["hard_maximum"]
            self.assertIsInstance(maximum, int, f"{row_id}: hard maximum must be finite integer")
            self.assertNotIsInstance(maximum, bool, f"{row_id}: boolean is not a numeric maximum")
            self.assertGreater(maximum, 0, f"{row_id}: hard maximum must be positive")
            unit = row["unit"]
            self.assertIsInstance(unit, str, f"{row_id}: unit must be a string")
            self.assertNotIn(unit.strip().lower(), ambiguous_units, f"{row_id}: unit is ambiguous")
            tests = row["boundary_tests"]
            self.assertIsInstance(tests, list, f"{row_id}: boundary_tests must be a list")
            self.assertTrue(tests, f"{row_id}: boundary_tests must be named")
            self.assertTrue(all(isinstance(item, str) and item.strip() for item in tests))
            configured = row["configurable_range"]
            self.assertIsInstance(configured, dict, f"{row_id}: configurable_range must be an object")
            self.assertEqual(maximum, configured.get("maximum"), f"{row_id}: range cannot exceed hard max")

    def test_final_supersession_values_are_the_only_values_for_corrected_ids(self) -> None:
        by_id = {str(row["id"]): row for row in self.actual}
        final_values = {
            "DUR04-FIRST-PROD-MANIFEST-FIELDS": 20,
            "DUR04-FIRST-PROD-DECODED-FIELDS": 8432,
            "DUR04-FIRST-PROD-SERVER-ARTIFACT-BYTES": 4304614,
            "DUR04-FIRST-PROD-CLIENT-ARTIFACT-BYTES": 34248,
            "DUR04-FIRST-PROD-GENERATION-PAIR-BYTES": 4338862,
            "DUR04-FIRST-PROD-SPAWN-POPULATION-PER-SPAWN": 1,
            "DUR04-FIRST-PROD-SPAWN-POPULATION-PER-SCOPE": 1,
        }
        for row_id, expected in final_values.items():
            self.assertEqual(expected, by_id[row_id]["hard_maximum"])

        for superseded in (4305510, 35144, 4340654):
            self.assertNotIn(str(superseded), self.registry_text)


if __name__ == "__main__":
    unittest.main()
