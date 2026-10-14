#!/usr/bin/env python3
"""Routing regressions for the Quest-authoring scope classifier and its workflow wiring."""
from __future__ import annotations

import hashlib
import re
import subprocess
import tempfile
import unittest
from pathlib import Path

import classify_quest_authoring_scope as scope

REPO = Path(__file__).resolve().parents[2]
TOOL = scope.TOOL_DIR


def rec(path, status="modified", previous=None):
    record = {"filename": path, "status": status}
    if previous:
        record["previous_filename"] = previous
    return record


class SyntheticTree(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.write(f"{TOOL}/README.md", "tool docs\n")
        self.write(f"{TOOL}/samples/a/README.md", "sample docs\n")
        self.write(f"{TOOL}/samples/a/data.json", '{"files": {}}\n')
        self.write(f"{TOOL}/maps/README.md", "pinned docs\n")
        self.write(f"{TOOL}/maps/qualification.json", '{"pins": {"README.md": "0"}}\n')
        self.write(f"{TOOL}/hashed/README.md", "hash only\n")
        digest = hashlib.sha256(b"hash only\n").hexdigest()
        self.write(f"{TOOL}/hashed/pins.json", '{"sha": "%s"}\n' % digest)
        self.write(f"{TOOL}/viaparent/README.md", "by parent path\n")
        self.write(f"{TOOL}/index.json", '{"doc": "viaparent/README.md"}\n')
        self.write(f"{TOOL}/run_checks.py", "print(1)\n")
        self.write("docs/note.md", "outside\n")

    def tearDown(self):
        self.tmp.cleanup()

    def write(self, rel, text):
        path = self.root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def verdict(self, records, **kw):
        return scope.classify(records, self.root, **kw)[0]

    def test_docs_only_quest_readme_skips_full_checks(self):
        self.assertEqual(self.verdict([rec(f"{TOOL}/README.md")]), "docs_only")
        self.assertEqual(
            self.verdict([rec(f"{TOOL}/README.md"), rec(f"{TOOL}/samples/a/README.md")]), "docs_only"
        )
        self.assertEqual(self.verdict([rec(f"{TOOL}/samples/missing.md", "added")]), "full")  # not on disk

    def test_added_unreferenced_doc_is_docs_only(self):
        self.write(f"{TOOL}/samples/a/NOTES.md", "new\n")
        self.assertEqual(self.verdict([rec(f"{TOOL}/samples/a/NOTES.md", "added")]), "docs_only")

    def test_readme_pin_change_takes_full_path(self):
        self.assertEqual(self.verdict([rec(f"{TOOL}/maps/README.md")]), "full")
        self.assertIn("qualification.json", scope.classify([rec(f"{TOOL}/maps/README.md")], self.root)[1])

    def test_content_hash_and_parent_path_pins_take_full_path(self):
        self.assertEqual(self.verdict([rec(f"{TOOL}/hashed/README.md")]), "full")
        self.assertEqual(self.verdict([rec(f"{TOOL}/viaparent/README.md")]), "full")

    def test_mixed_diff_takes_full_path(self):
        self.assertEqual(self.verdict([rec(f"{TOOL}/README.md"), rec(f"{TOOL}/run_checks.py")]), "full")
        self.assertEqual(self.verdict([rec(f"{TOOL}/README.md"), rec("docs/note.md")]), "full")
        self.assertEqual(self.verdict([rec(f"{TOOL}/README.md"), rec("apps/game-server/src/lib.rs")]), "full")

    def test_rename_and_delete_take_full_path(self):
        self.assertEqual(self.verdict([rec(f"{TOOL}/README.md", "removed")]), "full")
        self.assertEqual(self.verdict([rec(f"{TOOL}/NEW.md", "renamed", f"{TOOL}/README.md")]), "full")
        self.assertEqual(self.verdict([rec(f"{TOOL}/README.md", "modified", f"{TOOL}/OLD.md")]), "full")
        self.assertEqual(self.verdict([rec(f"{TOOL}/README.md", "copied")]), "full")

    def test_workflow_build_and_dependency_changes_take_full_path(self):
        for path in (
            ".github/workflows/quest-authoring-schema.yml",
            ".github/workflows/merge-gate.yml",
            "tools/content-schema/monster-authoring/requirements.txt",
            "Cargo.toml",
            "Cargo.lock",
            f"{TOOL}/run_checks.py",
            f"{TOOL}/quest_content.schema.json",
        ):
            self.assertEqual(self.verdict([rec(path)]), "full", path)
            self.assertEqual(self.verdict([rec(path), rec(f"{TOOL}/README.md")]), "full", path)

    def test_unknown_or_malformed_input_takes_full_path(self):
        for records in (None, [], {}, ["x"], [{"filename": f"{TOOL}/README.md"}], [rec(f"{TOOL}/README.MD")],
                        [rec(f"{TOOL}/../../../docs/note.md")], [rec(f"{TOOL}/README.md", "mystery")],
                        [rec("tools/content-schema/quest-authoring-x/README.md")]):
            self.assertEqual(self.verdict(records), "full", records)
        self.assertEqual(self.verdict([rec(f"{TOOL}/README.md")], complete=False), "full")
        self.assertEqual(self.verdict([rec(f"{TOOL}/README.md")] * scope.MAX_ENUMERATED), "full")


class RealTree(unittest.TestCase):
    """Run the classifier on the committed tree, where README pins are real."""

    def test_known_pinned_readmes_take_full_path(self):
        for rel in (
            "donor_sources/maps/README.md",
            "samples/server-completion/chosen-progress/README.md",
            "samples/server-completion/npc-dialogue/README.md",
            "samples/state-effect-refinements/README.md",
        ):
            self.assertEqual(scope.classify([rec(f"{TOOL}/{rel}")], REPO)[0], "full", rel)

    def test_unreferenced_tool_readme_is_docs_only(self):
        self.assertEqual(scope.classify([rec(f"{TOOL}/README.md")], REPO)[0], "docs_only")

    def test_every_ignored_doc_is_unreferenced(self):
        tracked = subprocess.run(
            ["git", "ls-files", "-z", "--", TOOL], cwd=REPO, check=True, capture_output=True
        ).stdout.decode().split("\0")
        docs = [path for path in tracked if path.endswith(".md")]
        self.assertTrue(docs)
        pinned = scope.referenced_docs(REPO, docs)
        for doc in docs:
            verdict = scope.classify([rec(doc)], REPO)[0]
            self.assertEqual(verdict == "full", doc in pinned, doc)
            if verdict == "docs_only":
                digest = hashlib.sha256((REPO / doc).read_bytes()).hexdigest().encode()
                for sibling in (REPO / doc).parent.iterdir():
                    if sibling.is_file() and sibling.suffix not in {".md", ".gz"}:
                        data = sibling.read_bytes()
                        self.assertNotIn(Path(doc).name.encode(), data, f"{doc} named in {sibling.name}")
                        self.assertNotIn(digest, data, f"{doc} hash in {sibling.name}")
        self.assertGreaterEqual(len(pinned), 4)


class WorkflowWiring(unittest.TestCase):
    def setUp(self):
        self.text = (REPO / ".github/workflows/quest-authoring-schema.yml").read_text(encoding="utf-8")

    def test_classifier_runs_before_the_expensive_steps_and_only_they_are_gated(self):
        steps = re.split(r"\n      - name: ", self.text)[1:]
        names = [step.split("\n", 1)[0] for step in steps]
        self.assertLess(names.index("Classify Quest scope"), names.index("Set up Python"))
        for step in steps:
            name = step.split("\n", 1)[0]
            gated = "steps.scope.outputs.full == 'true'" in step
            self.assertEqual(
                gated,
                name in {"Set up Python", "Install pinned schema requirements", "Validate all offline Quest data"},
                name,
            )

    def test_job_name_and_triggers_are_unchanged(self):
        self.assertIn("\n  quest-authoring:\n", self.text)
        self.assertIn("      - tools/content-schema/quest-authoring/**\n", self.text)
        self.assertNotIn("paths-ignore", self.text)
        self.assertNotIn("!tools/content-schema/quest-authoring/**/*.md", self.text)

    def test_classifier_comes_from_the_protected_base(self):
        self.assertIn("contents/tools/repository/classify_quest_authoring_scope.py?ref=$BASE_SHA", self.text)
        self.assertIn("github.event.pull_request.base.sha", self.text)
        self.assertIn("workflow_dispatch", self.text)


class MergeGateOrdering(unittest.TestCase):
    def jobs(self):
        text = (REPO / ".github/workflows/merge-gate.yml").read_text(encoding="utf-8")
        jobs = {}
        for block in re.split(r"\n(?=  [a-z_]+:\n)", text.split("\njobs:\n", 1)[1]):
            name = block.strip().split(":", 1)[0]
            match = re.search(r"^    needs: (.+)$", block, re.MULTILINE)
            jobs[name] = (block, match.group(1) if match else "")
        return jobs

    def test_heavy_rust_jobs_wait_for_cheap_policy_jobs(self):
        jobs = self.jobs()
        for heavy in ("rust_policy", "rust_fast", "rust_supply_chain"):
            for cheap in ("governance", "dependency_review", "routing_contract"):
                self.assertIn(cheap, jobs[heavy][1], f"{heavy} needs {cheap}")
        for cheap in ("governance", "dependency_review", "routing_contract"):
            self.assertEqual(jobs[cheap][1], "scope", f"{cheap} must stay a first-wave job")

    def test_required_aggregates_still_always_report(self):
        jobs = self.jobs()
        for name in ("validate", "game_gate"):
            self.assertIn("    if: always() &&", jobs[name][0])
        for needed in ("governance", "dependency_review", "routing_contract", "rust_fast", "world_bundle"):
            self.assertIn(needed, jobs["validate"][1])
        self.assertEqual(jobs["game_gate"][1], "[scope, validate]")


if __name__ == "__main__":
    unittest.main()
