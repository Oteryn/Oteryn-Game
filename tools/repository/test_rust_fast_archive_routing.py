#!/usr/bin/env python3
"""The shared archive producer contributes its exact-candidate helper consumers."""
from __future__ import annotations

import os
import tempfile
import unittest
from pathlib import Path, PurePosixPath

from test_classify_pr_test_lanes import classify, fixture, git, load_module


class RustFastArchiveRoutingTests(unittest.TestCase):
    def test_archive_only_helper_is_a_control_consumer(self):
        module = load_module()
        archive = ".github/workflows/rust-fast-archive.yml"
        helper = "tools/ci/archive_helper.py"
        unrelated = "tools/unrelated/helper.py"
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            git(root, "init", "-q")
            git(root, "config", "user.email", "ci@example.invalid")
            git(root, "config", "user.name", "CI")
            metadata = fixture()
            metadata_root = PurePosixPath(metadata["workspace_root"])
            for package in metadata["packages"]:
                relative = PurePosixPath(package["manifest_path"]).relative_to(metadata_root)
                manifest = root.joinpath(*relative.parts)
                manifest.parent.mkdir(parents=True, exist_ok=True)
                manifest.write_text("[package]\n", encoding="utf-8")
            for control in module.CANONICAL_CONTROL_PATHS | {archive}:
                workflow = root / control
                workflow.parent.mkdir(parents=True, exist_ok=True)
                workflow.write_text("name: control\n", encoding="utf-8")
            (root / ".github/workflows/merge-gate.yml").write_text(
                f"jobs:\n  archive:\n    uses: ./{archive}\n", encoding="utf-8"
            )
            (root / archive).write_text(
                "on: workflow_call\njobs:\n  build:\n    runs-on: ubuntu-latest\n"
                f"    steps:\n      - run: python3 {helper}\n",
                encoding="utf-8",
            )
            for relative in (helper, unrelated):
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("# fixture helper\n", encoding="utf-8")
            git(root, "add", ".")
            git(root, "commit", "-qm", "fixture")
            sha = git(root, "rev-parse", "HEAD")
            old_cwd = os.getcwd()
            os.chdir(root)
            try:
                found = module.candidate_reference_consumers(metadata, sha, [helper, unrelated])
            finally:
                os.chdir(old_cwd)
        self.assertEqual(found, {helper: {module.CONTROL_CONSUMER}, unrelated: set()})

    def test_real_workflows_keep_auxiliary_edits_narrow_and_unknown_selectors_full(self):
        module = load_module()
        repository = Path(__file__).resolve().parents[2]
        auxiliary = ".github/workflows/integration-regressions.yml"
        exact_consumers = (
            ".github/workflows/native-entry-room-qualification.yml",
            ".github/workflows/rust-fast-archive.yml",
        )
        paths = [auxiliary, *exact_consumers]
        workflows = {
            path: (repository / path).read_bytes()
            for path in module.CANONICAL_CONTROL_PATHS | set(paths)
        }
        for extra in (
            "",
            "cat .github/workflows/future-qualification.yml",
            "ls .github/workflows/*.yml",
            "cat .github/workflows/${{ inputs.workflow }}.yml",
        ):
            with self.subTest(reference=extra or "audited-only"):
                with tempfile.TemporaryDirectory() as directory:
                    root = Path(directory)
                    git(root, "init", "-q")
                    git(root, "config", "user.email", "ci@example.invalid")
                    git(root, "config", "user.name", "CI")
                    metadata = fixture()
                    metadata_root = PurePosixPath(metadata["workspace_root"])
                    for package in metadata["packages"]:
                        relative = PurePosixPath(package["manifest_path"]).relative_to(metadata_root)
                        manifest = root.joinpath(*relative.parts)
                        manifest.parent.mkdir(parents=True, exist_ok=True)
                        manifest.write_text("[package]\n", encoding="utf-8")
                    for relative, content in workflows.items():
                        workflow = root / relative
                        workflow.parent.mkdir(parents=True, exist_ok=True)
                        workflow.write_bytes(content)
                    if extra:
                        merge_gate = root / ".github/workflows/merge-gate.yml"
                        merge_gate.write_bytes(
                            merge_gate.read_bytes()
                            + ("\n  routing_guard_probe:\n    runs-on: ubuntu-24.04\n"
                               f"    steps:\n      - run: {extra}\n").encode("utf-8")
                        )
                    git(root, "add", ".")
                    git(root, "commit", "-qm", "actual workflow routing fixture")
                    sha = git(root, "rev-parse", "HEAD")
                    old_cwd = os.getcwd()
                    os.chdir(root)
                    try:
                        found = module.candidate_reference_consumers(metadata, sha, paths)
                    finally:
                        os.chdir(old_cwd)
                expected_auxiliary = {module.CONTROL_CONSUMER} if extra else set()
                self.assertEqual(found[auxiliary], expected_auxiliary)
                for path in paths:
                    expected_full = bool(extra) or path in exact_consumers
                    if expected_full:
                        self.assertEqual(found[path], {module.CONTROL_CONSUMER})
                    result = classify(module, [path], consumers={path: found[path]})
                    self.assertEqual((result["rust"], result["windows"]),
                                     (expected_full, expected_full))


if __name__ == "__main__":
    unittest.main()
