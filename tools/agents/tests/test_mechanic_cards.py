"""Regression coverage for mechanic closeout evidence and index links."""

from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path

VALIDATOR_PATH = Path(__file__).resolve().parents[1] / "validate_governance.py"
SPEC = importlib.util.spec_from_file_location("validate_governance", VALIDATOR_PATH)
assert SPEC is not None and SPEC.loader is not None
validator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validator)


class MechanicCardTests(unittest.TestCase):
    def setUp(self) -> None:
        self.original_root = validator.ROOT
        self.tempdir = tempfile.TemporaryDirectory()
        validator.ROOT = Path(self.tempdir.name)
        self.tasks = validator.ROOT / "docs/agents/tasks"
        self.tasks.mkdir(parents=True)
        self.index = self.tasks / "MECHANICS_STATUS.md"
        self.index.write_text("# Mechanics status\n", encoding="utf-8")

    def tearDown(self) -> None:
        validator.ROOT = self.original_root
        self.tempdir.cleanup()

    def test_terminal_mechanics_require_real_parity_heading_in_every_card_directory(self) -> None:
        for directory in ("", "active", "archive"):
            for status in ("DONE", "completed", "'done'", '"completed"'):
                for heading, expected_errors in (
                    ("", 1),
                    ("## Reference parity\n", 0),
                    ("## Reference parity   \n", 0),
                    ("## Reference parity later\n", 1),
                    ("```markdown\n## Reference parity\n```\n", 1),
                ):
                    with self.subTest(directory=directory, status=status, heading=heading):
                        card = self.tasks / directory / "CASE-1.md"
                        card.parent.mkdir(parents=True, exist_ok=True)
                        card.write_text(
                            f"# CASE-1\n\n```yaml\ntype: mechanic\nstatus: {status}\n```\n\n{heading}",
                            encoding="utf-8",
                        )
                        errors: list[str] = []
                        validator.validate_mechanic_cards(errors)
                        self.assertEqual(len(errors), expected_errors, errors)
                        if errors:
                            self.assertIn("must contain a Reference parity heading", errors[0])
                        card.unlink()

    def test_nonterminal_and_other_task_types_keep_existing_rules(self) -> None:
        for metadata in (
            "type: mechanic\nstatus: implementing",
            "type: audit\nstatus: DONE",
            "status: completed",
            "type: mechanic",
        ):
            with self.subTest(metadata=metadata):
                (self.tasks / "CASE-1.md").write_text(metadata, encoding="utf-8")
                errors: list[str] = []
                validator.validate_mechanic_cards(errors)
                self.assertEqual(errors, [])

    def test_index_links_resolve_relative_to_index(self) -> None:
        (self.tasks / "archive").mkdir()
        (self.tasks / "archive/CASE-1.md").touch()
        source = validator.ROOT / "crates/example.rs"
        source.parent.mkdir()
        source.touch()
        (self.tasks / "space name.md").touch()
        self.index.write_text(
            "[card](archive/CASE-1.md) [source](../../../crates/example.rs)\n"
            "[encoded](space%20name.md) [remote](https://example.com/reference)\n"
            "[section](#mechanics)\n",
            encoding="utf-8",
        )
        errors: list[str] = []
        validator.validate_mechanic_cards(errors)
        self.assertEqual(errors, [])

    def test_index_rejects_missing_and_outside_repository_file_links(self) -> None:
        self.index.write_text(
            "[missing](archive/MISSING.md) [outside](../../../../outside.md)\n",
            encoding="utf-8",
        )
        errors: list[str] = []
        validator.validate_mechanic_cards(errors)
        self.assertEqual(errors, [
            "MECHANICS_STATUS.md has unresolved local file link: archive/MISSING.md",
            "MECHANICS_STATUS.md has unresolved local file link: ../../../../outside.md",
        ])

    def test_missing_index_is_reported(self) -> None:
        self.index.unlink()
        errors: list[str] = []
        validator.validate_mechanic_cards(errors)
        self.assertEqual(errors, [
            "missing mechanic index: docs/agents/tasks/MECHANICS_STATUS.md",
        ])


if __name__ == "__main__":
    unittest.main()
