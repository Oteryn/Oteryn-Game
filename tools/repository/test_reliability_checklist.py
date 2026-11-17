"""The reliability checklist stays loadable, sourced and self-testing."""

from __future__ import annotations

import io
import re
import sys
import unittest
from contextlib import redirect_stdout
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import reliability_checklist as rc


def slug(heading: str) -> str:
    text = re.sub(r"[^\w\s-]", "", heading.strip().lower())
    return re.sub(r"\s", "-", text)


class ReliabilityChecklistTest(unittest.TestCase):
    def setUp(self) -> None:
        self.items = rc.load()

    def test_ids_unique_and_well_formed(self) -> None:
        ids = [item["id"] for item in self.items]
        self.assertEqual(len(ids), len(set(ids)))
        for item_id in ids:
            self.assertRegex(item_id, r"^[a-z0-9][a-z0-9-]{1,40}$")

    def test_regexes_compile(self) -> None:
        for item in self.items:
            for pattern in item["diff"]:
                re.compile(pattern)

    def test_every_source_exists(self) -> None:
        for item in self.items:
            path, _, anchor = item["source"].partition("#")
            source = rc.ROOT / path
            self.assertTrue(source.is_file(), item["id"])
            if anchor:
                headings = [
                    slug(line.lstrip("#"))
                    for line in source.read_text(encoding="utf-8").splitlines()
                    if line.startswith("#")
                ]
                self.assertIn(anchor, headings, item["id"])

    def test_every_example_triggers_its_item(self) -> None:
        for item in self.items:
            examples = item["examples"]
            self.assertTrue(examples["paths"] or examples["diff"], item["id"])
            for path in examples["paths"]:
                hits = [hit["id"] for hit, _ in rc.match([item], [path], [])]
                self.assertEqual(hits, [item["id"]], f"{item['id']}: {path}")
            for line in examples["diff"]:
                hits = [hit["id"] for hit, _ in rc.match([item], [], [line])]
                self.assertEqual(hits, [item["id"]], f"{item['id']}: {line}")

    def test_unrelated_change_matches_nothing(self) -> None:
        self.assertEqual(rc.match(self.items, ["README.md"], ["+Plain prose."]), [])

    def test_glob_is_anchored(self) -> None:
        self.assertFalse(rc.path_matches("content/**", "docs/content/x.md"))
        self.assertTrue(rc.path_matches("content/**", "content/a/b.json"))
        self.assertTrue(rc.path_matches("AGENTS.md", "AGENTS.md"))

    def test_diff_headers_are_not_changed_lines(self) -> None:
        diff = "--- a/x.sh\n+++ b/x.sh\n@@ -1 +1 @@\n-old | tee log\n+new\n"
        self.assertEqual(rc.changed_lines(diff), ["-old | tee log", "+new"])

    def test_cli_paths_mode(self) -> None:
        out = io.StringIO()
        with redirect_stdout(out):
            code = rc.main(["--paths", "apps/game-server/migrations/0999_x.sql"])
        self.assertEqual(code, 0)
        self.assertIn("[migration-number]", out.getvalue())
        self.assertIn("dup-migrations.sh", out.getvalue())


if __name__ == "__main__":
    unittest.main()
