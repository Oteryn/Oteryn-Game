"""Actual source carry retains every adopted row and rejects conflicting input."""

import copy
import json
import unittest
from unittest.mock import patch

import compose_monster_current_sources as compose
import import_current_spell_sources as current
import import_spell_families as importer
import monster_seven_spell_overlay as overlay


class ComposeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root = importer.ROOT
        cls.base = importer.baseline_outputs(cls.root)
        cls.current = current.outputs(cls.root, cls.base)
        cls.adopted = overlay.apply_overlay(cls.root, cls.base)

    def test_actual1351_source_changes_preserve_all1870_adopted_rows(self):
        before = copy.deepcopy(self.adopted)
        result = compose.compose(self.root, self.base, self.current)
        adopted = compose.indexed(json.loads(self.adopted[compose.CREATURES]))
        incoming = compose.indexed(json.loads(self.current[compose.CREATURES]))
        baseline = compose.indexed(json.loads(self.base[compose.CREATURES]))
        combined = compose.indexed(json.loads(result[compose.CREATURES]))
        self.assertEqual(len(combined), 1870)
        self.assertEqual(combined.keys(), adopted.keys())
        changed = 0
        for key, retained in adopted.items():
            self.assertEqual(
                {k: v for k, v in combined[key].items() if k != "monster_melee"},
                {k: v for k, v in retained.items() if k != "monster_melee"},
            )
            expected = retained.get("monster_melee")
            if key in incoming and incoming[key].get("monster_melee") != baseline[key].get("monster_melee"):
                expected = incoming[key]["monster_melee"]
                changed += 1
            self.assertEqual(combined[key].get("monster_melee"), expected)
        self.assertEqual(changed, 1351)
        self.assertEqual(self.adopted, before)
        for path in overlay.ALLOWED_PATHS - {compose.CREATURES, current.MANIFEST}:
            self.assertEqual(result[path], self.adopted[path])
        for section in ("catalog", "source_selection"):
            self.assertEqual(
                json.loads(result[current.MANIFEST])[section],
                json.loads(self.current[current.MANIFEST])[section],
            )

    def test_foreign_profile_changes_missing_rows_and_duplicates_are_rejected(self):
        for operator, message in (
            ("foreign_behavior", "NON_MELEE_CHANGE"),
            ("missing", "CURRENT_SCOPE_CHANGED"),
            ("duplicate", "DUPLICATE_IDENTITY"),
        ):
            document = json.loads(self.current[compose.CREATURES])
            if operator == "foreign_behavior":
                document["records"][0]["detached_health"] = 1
            elif operator == "missing":
                document["records"].pop()
            else:
                document["records"].append(copy.deepcopy(document["records"][0]))
            changed = self.current | {compose.CREATURES: importer.canonical_bytes(document)}
            with self.subTest(operator=operator), self.assertRaisesRegex(ValueError, message):
                compose.compose(self.root, self.base, changed)

    def test_conflicting_adopted_melee_cannot_be_silently_overwritten(self):
        document = json.loads(self.adopted[compose.CREATURES])
        incoming = compose.indexed(json.loads(self.current[compose.CREATURES]))
        baseline = compose.indexed(json.loads(self.base[compose.CREATURES]))
        row = next(
            r for r in document["records"]
            if compose.identity(r) in incoming
            and incoming[compose.identity(r)].get("monster_melee")
            != baseline[compose.identity(r)].get("monster_melee")
        )
        row["monster_melee"] = {"detached_source": True}
        negative = self.adopted | {compose.CREATURES: importer.canonical_bytes(document)}
        with patch.object(compose, "apply_overlay", return_value=negative):
            with self.assertRaisesRegex(ValueError, "MELEE_CONFLICT"):
                compose.compose(self.root, self.base, self.current)


if __name__ == "__main__":
    unittest.main()
