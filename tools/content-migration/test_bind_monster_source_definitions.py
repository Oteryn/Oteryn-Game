"""Source membership comes from an independently pinned definition closure."""
import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from bind_monster_source_definitions import bind_definitions, definition_digest, canonical, CREATURES, MANIFEST

class DefinitionBindingTests(unittest.TestCase):
    def fixture(self, root):
        values = {
            "definitions/reference.json": {"records": [{"identity": "rat", "health": 20}]},
            "definitions/declarations.json": {"authoring_profiles": [{"attack": 10}], "records": [{"encounter": "rat"}]},
            "provenance/sources.json": {"sources": [{"revision": "pinned"}], "source_identity_bindings": [{"target": "rat"}]},
        }
        for name, value in values.items():
            path = root / "content/world" / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(canonical(value))
        return values

    def test_binds_exact_definitions_and_updates_independent_native_input_pin(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.fixture(root)
            original = {CREATURES: canonical({"records": [{"profile": "original"}]}), MANIFEST: canonical({"creature_profiles": {"path": "original", "sha256": "old"}})}
            before = copy.deepcopy(original)
            bound = bind_definitions(root, original)
            self.assertEqual(original, before)
            self.assertEqual(json.loads(bound[CREATURES])["records"], json.loads(original[CREATURES])["records"])
            self.assertEqual(json.loads(bound[CREATURES])["source_definitions_sha256"], definition_digest(root))
            self.assertEqual(json.loads(bound[MANIFEST])["creature_profiles"]["sha256"], hashlib.sha256(bound[CREATURES]).hexdigest())

    def test_each_definition_or_provenance_substitution_changes_membership(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            values = self.fixture(root)
            expected = definition_digest(root)
            for name, original in values.items():
                for field in original:
                    with self.subTest(document=name, field=field):
                        altered = copy.deepcopy(original)
                        altered[field].append({"substitution": "detached"})
                        path = root / "content/world" / name
                        path.write_bytes(canonical(altered))
                        self.assertNotEqual(definition_digest(root), expected)
                        path.write_bytes(canonical(original))
            self.assertEqual(definition_digest(root), expected)

if __name__ == "__main__":
    unittest.main()
