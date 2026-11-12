import json
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator


class VisibilitySchemaTests(unittest.TestCase):
    def test_all_public_content_schemas_accept_only_public_or_hidden(self):
        root = Path(__file__).resolve().parents[2] / "tools/content-schema"
        schemas = [
            ("npc-authoring/npc.schema.json", None),
            ("area-authoring/area.schema.json", "Area"),
            ("quest-authoring/quest_content.schema.json", "quest"),
            ("quest-authoring/quest_tree.schema.json", "sourceQuest"),
            ("quest-authoring/quest_authored.schema.json", None),
        ]
        for path, definition in schemas:
            with self.subTest(schema=path):
                schema = json.loads((root / path).read_text())
                record = schema["$defs"][definition] if definition else schema
                visibility = record["properties"]["visibility"]
                self.assertEqual(visibility["default"], "public")
                self.assertNotIn("visibility", record.get("required", []))
                validator = Draft202012Validator(visibility)
                for valid in ("public", "hidden"):
                    self.assertTrue(validator.is_valid(valid))
                for invalid in ("secret", "PUBLIC", "", None, True, 1, []):
                    self.assertFalse(validator.is_valid(invalid))
