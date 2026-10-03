"""Tests for tools/agents/build_decision_index.py: subject ids, header allocation lines, conflicts."""
from __future__ import annotations

import importlib.util
import subprocess
import tempfile
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location(
    "build_decision_index", Path(__file__).resolve().parents[1] / "build_decision_index.py"
)
bdi = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(bdi)


class DecisionIndexTests(unittest.TestCase):
    def test_subject_ranges_and_lane_prefixes(self) -> None:
        self.assertEqual(bdi.decision_ids("docs: batch (D84-D87)"), ["D84", "D85", "D86", "D87"])
        self.assertEqual(bdi.decision_ids("MONSTER-D15 Bestiary"), ["MONSTER-D15"])

    def test_header_allocation_forms(self) -> None:
        cp = "# X\n\n- Answers:\n  - control-plane allocation D300 (#1622), under D296;\n\n## 1. Q\n"
        self.assertEqual(bdi.header_allocation(cp, "a.md"), "D300")
        alloc = "# X\n- Allocation: D286 (#1622 comment 1), an exception to D252.\n## 1\n"
        self.assertEqual(bdi.header_allocation(alloc, "b.md"), "D286")
        inline = "# X\n- Answers: control-plane allocation D292 (#1622) and a ruling.\n## 1\n"
        self.assertEqual(bdi.header_allocation(inline, "d.md"), "D292")

    def test_header_ignores_references_and_body(self) -> None:
        text = "# X\n- Builds on: owner decision D238.\n## 1\n- Allocation: D9\n"
        self.assertIsNone(bdi.header_allocation(text, "c.md"))

    def test_header_with_two_allocations_fails(self) -> None:
        text = "# X\n- Allocation: D286\n  - control-plane allocation D287 (#1622)\n## 1\n"
        with self.assertRaises(bdi.DecisionConflict):
            bdi.header_allocation(text, "d.md")

    def test_merge_adds_header_ids(self) -> None:
        self.assertEqual(bdi.merge_ids([], {"a.md": "D286"}, "#1"), ["D286"])
        self.assertEqual(bdi.merge_ids(["MONSTER-D15"], {"a.md": "D286"}, "#1"), ["MONSTER-D15", "D286"])
        self.assertEqual(bdi.merge_ids(["D292"], {"a.md": "D292"}, "#1"), ["D292"])

    def test_subject_and_header_disagree_fails(self) -> None:
        with self.assertRaises(bdi.DecisionConflict):
            bdi.merge_ids(["D292"], {"a.md": "D293"}, "#1")

    def test_documents_disagree_fails(self) -> None:
        with self.assertRaises(bdi.DecisionConflict):
            bdi.merge_ids([], {"a.md": "D292", "b.md": "D293"}, "#1")

    def test_lane_local_subject_does_not_hide_document_conflict(self) -> None:
        with self.assertRaises(bdi.DecisionConflict):
            bdi.merge_ids(["MONSTER-D15"], {"a.md": "D286", "b.md": "D287"}, "#1")

    def test_shallow_boundary_is_detected(self) -> None:
        def run(cwd: Path, *args: str) -> str:
            return subprocess.run(args, cwd=cwd, check=True, capture_output=True, text=True).stdout.strip()

        with tempfile.TemporaryDirectory() as tmp:
            src, clone = Path(tmp, "src"), Path(tmp, "clone")
            src.mkdir()
            run(src, "git", "init", "-q")
            for n in (1, 2):
                Path(src, "f").write_text(str(n))
                run(src, "git", "add", "f")
                run(src, "git", "-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", str(n))
            self.assertEqual(bdi.shallow_commits(src), set())
            run(Path(tmp), "git", "clone", "-q", "--depth", "1", src.as_uri(), "clone")
            self.assertEqual(bdi.shallow_commits(clone), {run(clone, "git", "rev-parse", "HEAD")})

    def test_shallow_boundary_merge_refuses_unless_indexed(self) -> None:
        def run(cwd: Path, *args: str) -> str:
            return subprocess.run(args, cwd=cwd, check=True, capture_output=True, text=True).stdout.strip()

        with tempfile.TemporaryDirectory() as tmp:
            src, clone = Path(tmp, "src"), Path(tmp, "clone")
            src.mkdir()
            run(src, "git", "init", "-q")
            for n, subject in ((1, "base"), (2, "docs(arch): decision (D9) (#7)")):
                Path(src, "f").write_text(str(n))
                run(src, "git", "add", "f")
                run(src, "git", "-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", subject)
            run(Path(tmp), "git", "clone", "-q", "--depth", "1", src.as_uri(), "clone")
            saved = bdi.ROOT
            bdi.ROOT = clone
            try:
                with self.assertRaises(bdi.ShallowHistory):
                    bdi.scan("HEAD")
                self.assertEqual(bdi.scan("HEAD", known={7}), {})
            finally:
                bdi.ROOT = saved


if __name__ == "__main__":
    unittest.main()
