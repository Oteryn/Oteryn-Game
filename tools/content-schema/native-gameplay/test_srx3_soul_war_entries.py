"""Source capture retains overlapping entries and Lua's last Greed field."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location("entries", Path(__file__).with_name("build_srx3_soul_war_entries.py"))
TOOL = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(TOOL)


class SoulWarEntryCaptureTests(unittest.TestCase):
    def test_actual_capture_preserves_six_source_recipes_and_shared_entry_cells(self):
        rows = json.loads(TOOL.OUTPUT.read_bytes())
        self.assertEqual(len(rows), 6)
        self.assertEqual({r["slug"] for r in rows}, {n.lower() + ("_purple" if n == "Megalomania" else "") for n in TOOL.NAMES})
        for row in rows:
            self.assertEqual(len(set(map(tuple, row["levers"]))), 5)
            self.assertEqual(len(row["entries"]), 5)
            self.assertEqual(len(set(map(tuple, row["entries"]))), 1)
        greed = next(r for r in rows if r["slug"] == "greed")
        self.assertEqual(greed["cooldown_seconds"], 72000)
        mega = next(r for r in rows if r["slug"] == "megalomania_purple")
        self.assertEqual(mega["cooldown_seconds"], 259200)
        self.assertEqual(mega["boss"], [33710, 31634, 14])

    def test_changed_pinned_source_cannot_be_recaptured_under_original_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / TOOL.SOURCE
            path.parent.mkdir(parents=True)
            path.write_bytes(b"changed source")
            with self.assertRaisesRegex(ValueError, "pinned Soul War library changed"):
                TOOL.build(root)


if __name__ == "__main__":
    unittest.main()
