import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest


SPEC = importlib.util.spec_from_file_location("refresh_pins", Path(__file__).with_name("refresh_ci_workflow_pins.py"))
pins = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(pins)


class RotationTests(unittest.TestCase):
    def test_generated_rotation_is_complete_idempotent_and_preserves_legacy(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            paths = set(pins.REFERENCES) | set(pins.SOURCES) | {pins.AUDIT, pins.CONTRACT_TEST, ".github/workflows/merge-gate.yml"}
            for relative in paths:
                target = root / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes((pins.ROOT / relative).read_bytes())
            pr_harness = root / ".github/workflows/pr-postgres-harness.yml"
            legacy = [line for line in pr_harness.read_text().splitlines() if "readonly LEGACY_" in line]
            self.assertEqual({line.strip().split("=")[0] for line in legacy},
                             {"readonly LEGACY_HELPER_SHA", "readonly LEGACY_BUILD_SHA", "readonly LEGACY_PLANNER_SHA"})
            for relative in pins.SOURCES:
                with (root / relative).open("a") as stream:
                    stream.write("\n# Changed source for a new reviewed candidate.\n")
            changes = pins.proposal(root)
            self.assertIn(pins.CORE, changes)
            self.assertIn(pins.AUDIT, changes)
            for relative, content in changes.items():
                (root / relative).write_text(content)
            self.assertEqual(pins.proposal(root), {})
            self.assertEqual([line for line in pr_harness.read_text().splitlines() if "readonly LEGACY_" in line], legacy)
            gate = (root / ".github/workflows/merge-group-gate.yml").read_bytes()
            actual_git = subprocess.check_output(["git", "hash-object", "--stdin"], input=gate).decode().strip()
            self.assertIn(f'EXPECTED_MERGE_GROUP_GATE_BLOB: "{actual_git}"', (root / pins.AUDIT).read_text())
            self.assertIn(f'EXPECTED_MERGE_GROUP_GATE_BLOB = "{actual_git}"', (root / pins.CORE).read_text())

    def test_ambiguous_or_missing_authority_pin_fails(self):
        for text in ('EXPECTED_MERGE_GATE_BLOB = "bad"\n', 'nothing\n',
                     ('EXPECTED_MERGE_GATE_BLOB = "' + "a" * 40 + '"\n') * 2):
            with self.subTest(text=text), self.assertRaises(ValueError):
                pins.replace_constant(text, "EXPECTED_MERGE_GATE_BLOB", "b" * 40)


if __name__ == "__main__":
    unittest.main()
