"""Actual source carry retains every adopted row and rejects conflicting input."""

import copy
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import compose_monster_current_sources as compose
import import_current_spell_sources as current
import import_spell_families as importer
import monster_seven_spell_overlay as overlay
from bind_monster_source_definitions import bind_definitions


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
        # The pre-existing final binder supplies canonical attack closures and
        # explicit nullable MRL defaults. Preserve the entire qualified record.
        expected_adopted = json.loads(bind_definitions(self.root, self.adopted)[compose.CREATURES])
        adopted = compose.indexed(expected_adopted)
        incoming = compose.indexed(json.loads(self.current[compose.CREATURES]))
        baseline = compose.indexed(json.loads(self.base[compose.CREATURES]))
        combined = compose.indexed(json.loads(result[compose.CREATURES]))
        additions = {"oteryn:creature.cake_golem"}
        presentations = ["oteryn:presentation.creature.cake_golem"]
        references = compose.read_project_document(self.root, "definitions/reference.json")
        if any(row["identity"]["key"] == compose.ORC_SAM_CREATURE for row in references["records"]):
            additions.add(compose.ORC_SAM_CREATURE)
            presentations.append("oteryn:presentation.creature.orc_sam_backpack")
        self.assertEqual(len(combined), 1870 + len(additions))
        self.assertEqual(
            {combined[key]["profile"]["target"]["key"] for key in set(combined) - set(adopted)},
            additions,
        )
        self.assertTrue(adopted.keys() <= combined.keys())
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
        for path in overlay.ALLOWED_PATHS - {compose.CREATURES, current.MANIFEST,
                                              "content/presentations/definitions/spell-native-profiles.json"}:
            self.assertEqual(result[path], self.adopted[path])
        presentation_path = "content/presentations/definitions/spell-native-profiles.json"
        old_presentations = json.loads(self.adopted[presentation_path])["records"]
        new_presentations = json.loads(result[presentation_path])["records"]
        self.assertEqual(new_presentations[:len(old_presentations)], old_presentations)
        self.assertEqual([r["target"]["key"] for r in new_presentations[len(old_presentations):]],
                         presentations)
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


class OrcSamProjectionTests(unittest.TestCase):
    def test_admission_preserves_projected_nullable_reference_level(self):
        packet = compose.load(importer.ROOT, compose.ORC_SAM_SOURCE)
        raw_creature = packet["profiles"][0]
        projected = copy.deepcopy(raw_creature["data"])
        projected["profile"]["details"]["reference_level"] = None
        shard = {"records": [{"definition": packet["records"][0], "authoring": projected}]}
        documents = {
            "definitions/reference.json": {"records": packet["records"]},
            "definitions/declarations.json": {"authoring_profiles": packet["profiles"]},
        }
        generated = {
            compose.CREATURES: json.dumps({"schema": "OTERYN_NATIVE_CREATURE_PROFILES/v1", "records": []}),
            compose.PRESENTATIONS: json.dumps({
                "schema": "OTERYN_NATIVE_PRESENTATION_PROFILES/v1", "records": [],
            }),
            "content/spells.manifest.json": json.dumps({"presentation_profiles": {}}),
        }
        with (
            patch.object(Path, "glob", return_value=[Path("creatures-shard.json")]),
            patch.object(compose, "read_derived_shard", return_value=shard),
            patch.object(compose, "read_project_document", side_effect=lambda root, name: documents[name]),
        ):
            result = compose.admit_orc_sam(importer.ROOT, generated)
            record, = json.loads(result[compose.CREATURES])["records"]
            self.assertEqual(record["profile"]["data"], projected)
            self.assertNotIn("reference_level", raw_creature["data"]["profile"]["details"])
            projected["profile"]["health"] += 1
            with self.assertRaisesRegex(ValueError, "ORC_SAM_CANONICAL_CREATURE"):
                compose.admit_orc_sam(importer.ROOT, generated)


if __name__ == "__main__":
    unittest.main()
