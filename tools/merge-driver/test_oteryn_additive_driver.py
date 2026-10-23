#!/usr/bin/env python3
"""Tests for `oteryn_additive_driver.py`: hunk parsing, and a real `git merge` through the driver in a
temp repository. No cargo, no network."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from oteryn_additive_driver import combine  # noqa: E402

BASE = "mod a;\nmod z;\n"


def hunk(ours: str, base: str, theirs: str) -> str:
    return f"<<<<<<< ours\n{ours}||||||| base\n{base}=======\n{theirs}>>>>>>> incoming\n"


class CombineTest(unittest.TestCase):
    def test_insert_only_hunk_keeps_incoming_then_ours(self) -> None:
        text, combined, remaining = combine("mod a;\n" + hunk("mod b;\n", "", "mod c;\n") + "mod z;\n", 7)
        self.assertEqual(text, "mod a;\nmod c;\nmod b;\nmod z;\n")
        self.assertEqual((combined, remaining), (1, 0))

    def test_identical_insertions_are_kept_once(self) -> None:
        text, combined, remaining = combine(hunk("mod b;\n", "", "mod b;\n"), 7)
        self.assertEqual(text, "mod b;\n")
        self.assertEqual((combined, remaining), (1, 0))

    def test_shared_lines_inside_a_block_are_not_deduplicated(self) -> None:
        # git slid both appended functions across the closing `}` of the one before them.
        merged = "fn f() {\n" + hunk("}\nfn g() {\n", "", "}\nfn h() {\n") + "}\n"
        text, _, remaining = combine(merged, 7)
        self.assertEqual(text, "fn f() {\n}\nfn h() {\n}\nfn g() {\n}\n")
        self.assertEqual(remaining, 0)

    def test_edit_conflict_keeps_markers(self) -> None:
        merged = hunk("mod b;\n", "mod a;\n", "mod c;\n")
        text, combined, remaining = combine(merged, 7)
        self.assertEqual(text, merged)
        self.assertEqual((combined, remaining), (0, 1))

    def test_mixed_hunks_resolve_only_the_insert_only_one(self) -> None:
        edit = hunk("x = 1;\n", "x = 0;\n", "x = 2;\n")
        text, combined, remaining = combine(hunk("mod b;\n", "", "mod c;\n") + "mid\n" + edit, 7)
        self.assertEqual(text, "mod c;\nmod b;\nmid\n" + edit)
        self.assertEqual((combined, remaining), (1, 1))

    def test_unterminated_hunk_is_left_untouched(self) -> None:
        merged = "<<<<<<< ours\nmod b;\n||||||| base\n=======\nmod c;\n"
        text, combined, remaining = combine(merged, 7)
        self.assertEqual(text, merged)
        self.assertEqual((combined, remaining), (0, 1))

    def test_marker_size_is_honoured(self) -> None:
        merged = "<<<<<<<<<< ours\nb\n|||||||||| base\n==========\nc\n>>>>>>>>>> incoming\n"
        self.assertEqual(combine(merged, 10)[0], "c\nb\n")
        self.assertEqual(combine(merged, 7), (merged, 0, 0))


class GitMergeTest(unittest.TestCase):
    """Drives a real merge with the driver registered as the repository's install.sh registers it."""

    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.repo = Path(self.tmp.name)
        self.git("init", "-q", "-b", "main")
        self.git("config", "merge.oteryn-additive.driver", f"{sys.executable} {HERE / 'oteryn_additive_driver.py'} %O %A %B %L %P")
        (self.repo / ".gitattributes").write_text("*.rs merge=oteryn-additive\n*.json merge=oteryn-additive\n")
        (self.repo / "lib.rs").write_text(BASE)
        self.commit("base")
        self.git("checkout", "-q", "-b", "feature")

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def git(self, *args: str) -> subprocess.CompletedProcess[str]:
        identity = ["-c", "user.name=test", "-c", "user.email=test@example.invalid", "-c", "commit.gpgsign=false"]
        return subprocess.run(["git", *identity, *args], cwd=self.repo, capture_output=True, text=True)

    def commit(self, message: str) -> None:
        self.git("add", "-A")
        self.assertEqual(self.git("commit", "-q", "-m", message).returncode, 0)

    def branch_edit(self, branch: str, text: str, name: str = "lib.rs") -> None:
        self.git("checkout", "-q", branch)
        (self.repo / name).write_text(text)
        self.commit(branch)

    def merge_main(self) -> subprocess.CompletedProcess[str]:
        self.git("checkout", "-q", "feature")
        return self.git("merge", "-q", "--no-edit", "main")

    def test_parallel_mod_lines_merge_without_a_person(self) -> None:
        self.branch_edit("feature", "mod a;\nmod feature;\nmod z;\n")
        self.branch_edit("main", "mod a;\nmod incoming;\nmod z;\n")
        result = self.merge_main()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual((self.repo / "lib.rs").read_text(), "mod a;\nmod incoming;\nmod feature;\nmod z;\n")

    def test_parallel_edits_still_stop_the_merge(self) -> None:
        self.branch_edit("feature", "mod a2;\nmod z;\n")
        self.branch_edit("main", "mod a3;\nmod z;\n")
        self.assertNotEqual(self.merge_main().returncode, 0)
        text = (self.repo / "lib.rs").read_text()
        self.assertIn("<<<<<<< ours\nmod a2;\n||||||| base\nmod a;\n=======\nmod a3;\n>>>>>>> incoming\n", text)

    def registry(self, *ids: str) -> str:
        entries = ",\n".join(f'    {{\n      "id": "{i}",\n      "notes": "n"\n    }}' for i in ids)
        return f'{{\n  "entries": [\n{entries}\n  ]\n}}\n'

    def test_entries_appended_to_one_registry_array_both_land(self) -> None:
        self.branch_edit("main", self.registry("base"), "registry.json")
        self.git("checkout", "-q", "feature")
        self.git("merge", "-q", "main")
        self.branch_edit("feature", self.registry("base", "feature"), "registry.json")
        self.branch_edit("main", self.registry("base", "incoming"), "registry.json")
        result = self.merge_main()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        merged = json.loads((self.repo / "registry.json").read_text())
        self.assertEqual([e["id"] for e in merged["entries"]], ["base", "incoming", "feature"])

    def test_json_that_would_not_parse_keeps_the_conflict(self) -> None:
        self.branch_edit("main", '{\n  "a": 1\n}\n', "registry.json")
        self.git("checkout", "-q", "feature")
        self.git("merge", "-q", "main")
        self.branch_edit("feature", '{\n  "a": 1\n  "b": 2\n}\n', "registry.json")
        self.branch_edit("main", '{\n  "a": 1\n  "c": 3\n}\n', "registry.json")
        self.assertNotEqual(self.merge_main().returncode, 0)
        self.assertIn("<<<<<<< ours", (self.repo / "registry.json").read_text())


if __name__ == "__main__":
    unittest.main()
