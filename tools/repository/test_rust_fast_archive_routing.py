#!/usr/bin/env python3
"""The shared archive producer contributes its exact-candidate helper consumers."""
from __future__ import annotations

import os
import tempfile
import unittest
from pathlib import Path, PurePosixPath

from test_classify_pr_test_lanes import fixture, git, load_module


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


if __name__ == "__main__":
    unittest.main()
