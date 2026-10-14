#!/usr/bin/env python3
"""Routing regressions for the Quest-authoring scope classifier and its workflow wiring."""
from __future__ import annotations

import hashlib
import json
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

    def test_stale_pin_of_base_content_takes_full_path(self):
        # The pin still matches the base content; the head README changed without updating it.
        old = b"old readme\n"
        self.write(f"{TOOL}/stale/README.md", "new readme\n")
        self.write(f"{TOOL}/stale/pins.json", '{"sha": "%s"}\n' % hashlib.sha256(old).hexdigest())
        records = [rec(f"{TOOL}/stale/README.md")]
        self.assertEqual(scope.classify(records, self.root)[0], "docs_only")  # head-only view misses it
        base = {f"{TOOL}/stale/README.md": old}
        self.assertEqual(scope.classify(records, self.root, base_blobs=base)[0], "full")

    def test_hostile_file_name_cannot_inject_github_output(self):
        import json, os
        from unittest import mock
        name = f"{TOOL}/x\nfull=false\n.md"
        records = self.root / "records.json"
        records.write_text(json.dumps([rec(name)]), encoding="utf-8")
        out = self.root / "out.txt"
        with mock.patch.dict(os.environ, {"GITHUB_OUTPUT": str(out)}):
            scope.main(["--root", str(self.root), "--records", str(records)])
        lines = out.read_text(encoding="utf-8").splitlines()
        self.assertEqual(len(lines), 2, lines)
        self.assertEqual(lines[0], "full=true")
        self.assertTrue(lines[1].startswith("reason="))

    def test_dot_slash_and_dotdot_references_take_full_path(self):
        self.write(f"{TOOL}/dots/README.md", "d\n")
        self.write(f"{TOOL}/dots/pins.json", '{"doc": "./README.md"}\n')
        self.write(f"{TOOL}/dots2/README.md", "d\n")
        self.write(f"{TOOL}/dots2/pins.json", '{"doc": "x/../README.md"}\n')
        self.write(f"{TOOL}/dots3/README.md", "d\n")
        self.write(f"{TOOL}/dots3/sub/pins.json", '{"doc": "../README.md"}\n')
        self.write(f"{TOOL}/dots4/README.md", "d\n")
        self.write(f"{TOOL}/dots4/pins.json", f'{{"doc": "./{TOOL}/dots4/README.md"}}\n')
        for d in ("dots", "dots2", "dots3", "dots4"):
            self.assertEqual(self.verdict([rec(f"{TOOL}/{d}/README.md")]), "full", d)

    def test_root_absolute_and_odd_slash_references_take_full_path(self):
        forms = ["/{p}", "//{p}", "file:///{p}", "/./{p}", "/{d}/../{b}/README.md", ".\\{b}\\README.md"]
        for i, form in enumerate(forms):
            d = f"abs{i}"
            doc = f"{TOOL}/{d}/README.md"
            self.write(doc, "d\n")
            ref = form.format(p=doc, d=f"{TOOL}/{d}", b=d)
            if form.startswith("/{d}"):
                ref = f"/{TOOL}/x/../{d}/README.md"
            self.write(f"{TOOL}/{d}/pins.json", json.dumps({"doc": ref}))
            self.assertEqual(self.verdict([rec(doc)]), "full", ref)
        # An absolute path to a different file does not pin this one.
        self.write(f"{TOOL}/abs_ok/README.md", "d\n")
        self.write(f"{TOOL}/abs_ok/other.json", json.dumps({"doc": f"/{TOOL}/other/README.md"}))
        self.assertEqual(self.verdict([rec(f"{TOOL}/abs_ok/README.md")]), "docs_only")

    def test_odd_alphabet_references_take_full_path(self):
        for i, name in enumerate(("my doc.md", "a+b@c.md", "x y/README.md")):
            d = f"odd{i}"
            self.write(f"{TOOL}/{d}/{name}", "d\n")
            self.write(f"{TOOL}/{d}/pins.json", json.dumps({"doc": f"./{name}"}))
            self.write(f"{TOOL}/{d}/pins2.txt", f"see {name} for details\n")
            self.assertEqual(self.verdict([rec(f"{TOOL}/{d}/{name}")]), "full", name)
        self.write(f"{TOOL}/enc/my doc.md", "d\n")
        self.write(f"{TOOL}/enc/pins.json", '{"doc": "./my%20doc.md"}')
        self.assertEqual(self.verdict([rec(f"{TOOL}/enc/my doc.md")]), "full")
        self.write(f"{TOOL}/enc2/a+b@c.md", "d\n")
        self.write(f"{TOOL}/enc2/pins.json", '{"doc": "a%2bb%40c.md"}')
        self.assertEqual(self.verdict([rec(f"{TOOL}/enc2/a+b@c.md")]), "full")
        self.write(f"{TOOL}/enc3/my doc.md", "d\n")
        self.write(f"{TOOL}/enc3/pins.json", '{"doc": "my+doc.md"}')
        self.assertEqual(self.verdict([rec(f"{TOOL}/enc3/my doc.md")]), "full")
        self.write(f"{TOOL}/odd9/my doc.md", "d\n")
        self.write(f"{TOOL}/odd9/notes.txt", "mention of other my doc.mdx only\n")
        self.assertEqual(self.verdict([rec(f"{TOOL}/odd9/my doc.md")]), "docs_only")
        self.write(f"{TOOL}/odd8/caf\u00e9.md", "d\n")
        self.assertEqual(self.verdict([rec(f"{TOOL}/odd8/caf\u00e9.md")]), "full")

    def test_symlinked_doc_takes_full_path(self):
        (self.root / TOOL / "link.md").symlink_to(self.root / "docs/note.md")
        self.assertEqual(self.verdict([rec(f"{TOOL}/link.md", "added")]), "full")

    def test_moved_pull_request_fails_closed(self):
        import os
        from unittest import mock
        out = self.root / "out2.txt"
        env = {"GITHUB_OUTPUT": str(out), "GH_TOKEN": "x"}
        argv = ["--root", str(self.root), "--repository", "o/r", "--pull-request", "1",
                "--base-sha", "b" * 40, "--head-sha", "a" * 40]
        files = ([rec(f"{TOOL}/README.md")], True)
        cases = {
            "head moved before": [("c" * 40, "b" * 40)],
            "head moved after": [("a" * 40, "b" * 40), ("c" * 40, "b" * 40)],
            "base moved after": [("a" * 40, "b" * 40), ("a" * 40, "d" * 40)],
        }
        for name, live in cases.items():
            with mock.patch.dict(os.environ, env), \
                 mock.patch.object(scope, "live_pull", side_effect=live), \
                 mock.patch.object(scope, "fetch_records", return_value=files), \
                 mock.patch.object(scope, "fetch_base_blobs", return_value={}):
                out.write_text("")
                scope.main(argv)
            self.assertIn("full=true", out.read_text(), name)
        # Stable head and base: the unreferenced doc may skip.
        stable = [("a" * 40, "b" * 40)] * 2
        with mock.patch.dict(os.environ, env), \
             mock.patch.object(scope, "live_pull", side_effect=stable), \
             mock.patch.object(scope, "fetch_records", return_value=files), \
             mock.patch.object(scope, "fetch_base_blobs", return_value={}):
            out.write_text("")
            scope.main(argv)
        self.assertIn("full=false", out.read_text())
        # Missing expected SHAs also fail closed.
        with mock.patch.dict(os.environ, env):
            out.write_text("")
            scope.main(argv[:-2])
        self.assertIn("full=true", out.read_text())

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
        self.assertIn("--base-sha", self.text)
        self.assertIn("--head-sha", self.text)


if __name__ == "__main__":
    unittest.main()
