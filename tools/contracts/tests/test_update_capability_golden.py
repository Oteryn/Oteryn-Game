"""Consolidating a merged capability preserves the canonical ABI facts."""
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


class CapabilityConsolidation(unittest.TestCase):
    def check_consolidation(self, existing, previous_name="ITEM_EFFECTS_V1"):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            script = root / "tools/contracts/update_capability_golden.py"
            script.parent.mkdir(parents=True)
            shutil.copyfile(Path(__file__).parents[1] / script.name, script)
            package = root / "crates/protocol-oteryn"
            fragments = package / "capabilities"
            goldens = package / "src/tests/capabilities_golden"
            fragments.mkdir(parents=True)
            goldens.mkdir(parents=True)
            entry = {"id": 350, "name": "ITEM_EFFECTS_V1", "domains": [104]}
            filename = "0350-item-effects-v1.json"
            (fragments / filename).write_text(json.dumps(entry))
            target = goldens / filename
            target.write_text(json.dumps(existing))
            previous = goldens / "0385-item-effects-v1.json"
            previous.write_text(json.dumps({"id": 385, "name": previous_name}))
            original_target, original_previous = target.read_bytes(), previous.read_bytes()
            result = subprocess.run(
                [sys.executable, str(script), filename, "--previous", previous.name],
                capture_output=True,
                text=True,
            )
            if existing == entry and previous_name == entry["name"]:
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(json.loads(target.read_bytes()), entry)
                self.assertFalse(previous.exists())
            else:
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(target.read_bytes(), original_target)
                self.assertEqual(previous.read_bytes(), original_previous)

    def test_identical_existing_canonical_golden_consolidates(self):
        self.check_consolidation({"id": 350, "name": "ITEM_EFFECTS_V1", "domains": [104]})

    def test_existing_different_domain_cannot_be_overwritten(self):
        self.check_consolidation({"id": 350, "name": "ITEM_EFFECTS_V1", "domains": [105]})

    def test_previous_different_capability_cannot_be_removed(self):
        self.check_consolidation(
            {"id": 350, "name": "ITEM_EFFECTS_V1", "domains": [104]}, "OTHER_V1"
        )


if __name__ == "__main__":
    unittest.main()
