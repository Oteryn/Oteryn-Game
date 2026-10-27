"""MRL generated-data and no-staging regression tests; no Git mutations."""
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import regenerate_content as regeneration

ROOT = Path(__file__).resolve().parents[2]


class MonsterReferenceLevelTests(unittest.TestCase):
    def test_every_generated_creature_has_explicit_nullable_positive_cap(self):
        count = 0
        for path in sorted((ROOT / "content/creatures/definitions").glob("creatures-*.json")):
            for record in json.loads(path.read_text())["records"]:
                details = record["authoring"]["profile"]["details"]
                self.assertIn("reference_level", details, record["definition"]["identity"])
                cap = details["reference_level"]
                self.assertTrue(cap is None or type(cap) is int and cap > 0)
                self.assertFalse(details.get("reference_level_overrides"))
                count += 1
        self.assertGreater(count, 1000)
        native = json.loads((ROOT / "content/creatures/definitions/spell-native-profiles.json").read_text())
        for record in native["records"]:
            details = record["profile"]["data"]["profile"]["details"]
            self.assertIn("reference_level", details)
            self.assertIsNone(details["reference_level"])
            self.assertFalse(details.get("reference_level_overrides"))

    def test_summer_closure_accepts_explicit_null_but_rejects_a_changed_cap(self):
        import copy
        import summer_source_qualification as summer
        from bind_monster_source_definitions import explicit_reference_levels
        native = json.loads((ROOT / "content/creatures/definitions/spell-native-profiles.json").read_text())
        record = copy.deepcopy(next(record for record in native["records"]
                                    if record["profile"]["target"]["key"] == "oteryn:creature.radiant_acolyte"))
        before = summer.attack_definitions(ROOT, record)
        explicit_reference_levels(record)
        closure = summer.attack_definitions(ROOT, record)
        self.assertEqual(closure, before)
        self.assertTrue(closure["profiles"])
        record["profile"]["data"]["profile"]["details"]["reference_level"] = 1
        with self.assertRaisesRegex(ValueError, "Summer attack closure differs"):
            summer.attack_definitions(ROOT, record)

    def test_creature_roundtrip_requires_the_explicit_default(self):
        import copy
        import validate_world_project_v2_to_tree as validation
        reference = json.loads((ROOT / "content/world/definitions/reference.json").read_text())
        declarations = json.loads((ROOT / "content/world/definitions/declarations.json").read_text())
        sources = json.loads((ROOT / "content/world/provenance/sources.json").read_text())
        validation.validate_creature_families(reference, declarations, sources)
        original = validation.load

        def missing_cap(path):
            value = original(path)
            if path.parent == ROOT / "content/creatures/definitions" and path.name.startswith("creatures-"):
                value = copy.deepcopy(value)
                value["records"][0]["authoring"]["profile"]["details"].pop("reference_level")
            return value

        with patch.object(validation, "load", side_effect=missing_cap):
            with self.assertRaisesRegex(validation.ValidationError, "CREATURE_REFERENCE_LEVEL"):
                validation.validate_creature_families(reference, declarations, sources)

    def test_resolve_no_stage_never_mutates_git_index(self):
        commands = []
        with patch("sys.argv", ["regenerate_content.py", "--resolve", "--skip-checks", "--no-stage"]), \
                patch.object(regeneration, "resolve_conflicts", return_value=[]), \
                patch.object(regeneration, "materialize_world"), \
                patch.object(regeneration, "repin_rust_inventory"), \
                patch.object(regeneration, "derived_paths", return_value=set()), \
                patch.object(regeneration, "git", return_value=""), \
                patch.object(regeneration, "run", side_effect=commands.append):
            self.assertEqual(regeneration.main(), 0)
        self.assertTrue(any("pin-refresh" in command for command in commands))
        self.assertFalse(any(command[0] == "git" for command in commands))


if __name__ == "__main__":
    unittest.main()
