from __future__ import annotations

import os
import shlex
import subprocess
import sys
import tempfile
import unittest
from datetime import datetime, timezone
from pathlib import Path
from textwrap import dedent
from unittest.mock import patch

from producer import load_json_file, verify_snapshot


class CatalogueBuildIsolationTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.directory = Path(temporary.name)
        self.root = Path(__file__).resolve().parents[2]
        self.other = self.directory / "other"
        self.expected_ruleset = "oteryn:ruleset.public-catalog-candidate"
        subprocess.run(
            [
                "git",
                "-c",
                "core.hooksPath=/dev/null",
                "clone",
                "--quiet",
                "--shared",
                self.root,
                self.other,
            ],
            check=True,
        )
        self.revision = self.git("rev-parse", "HEAD")
        timestamp = self.git("show", "-s", "--format=%cI", "HEAD")
        self.timestamp = (
            datetime.fromisoformat(timestamp)
            .astimezone(timezone.utc)
            .strftime("%Y-%m-%dT%H:%M:%SZ")
        )
        projection = (
            self.other / "crates/gameplay/content/src/content/project/public_view.rs"
        )
        source = projection.read_text(encoding="utf-8")
        self.assertIn(self.expected_ruleset, source)
        projection.write_text(
            source.replace(
                self.expected_ruleset, "oteryn:ruleset.catalog-isolation-regression"
            ),
            encoding="utf-8",
        )
        subprocess.run(
            [
                "git",
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "user.name=Catalogue regression",
                "-c",
                "user.email=regression@example.invalid",
                "commit",
                "--quiet",
                "-am",
                "Change catalogue projection",
            ],
            cwd=self.other,
            env={**os.environ, "GIT_COMMITTER_DATE": "2000-01-01T00:00:00Z"},
            check=True,
        )

    def git(self, *args):
        return subprocess.check_output(
            ["git", "-c", "core.hooksPath=/dev/null", *args], cwd=self.other, text=True
        ).strip()

    def assert_pinned_build(self, root):
        from build import build

        for preproduction in (False, True):
            with self.subTest(preproduction=preproduction):
                output = self.directory / "catalog.json"
                build(root, output, preproduction=preproduction)
                snapshot = load_json_file(output)
                verify_snapshot(snapshot)
                self.assertEqual(
                    (
                        snapshot["source_revision"],
                        snapshot["generated_at"],
                        snapshot["ruleset_id"],
                    ),
                    (self.revision, self.timestamp, self.expected_ruleset),
                )

    def test_build_ignores_git_commit_replacements(self):
        replacement = self.git("rev-parse", "HEAD")
        self.git("switch", "--quiet", "--detach", self.revision)
        self.git("replace", self.revision, replacement)
        self.assert_pinned_build(self.other)

    def test_build_launches_its_own_cargo_executable(self):
        host = subprocess.check_output(
            ["rustc", "--print", "host-tuple"], text=True
        ).strip()
        target = self.directory / "caller-target"
        runner = self.directory / "runner.py"
        # Cargo invokes the runner after the pinned build releases its lock.
        # Rebuild from the competing checkout before launching the selected binary.
        runner.write_text(
            dedent("""\
                import os
                import subprocess
                import sys

                other, target, executable, *args = sys.argv[1:]
                subprocess.run(
                    ["cargo", "build", "--locked", "--quiet", "-p", "oteryn-gameplay-content",
                     "--bin", "oteryn-game-platform-catalog", "--target-dir", target],
                    cwd=other, check=True,
                )
                os.execv(executable, [executable, *args])
                """),
            encoding="utf-8",
        )
        runner_var = f"CARGO_TARGET_{host.upper().replace('-', '_')}_RUNNER"
        with patch.dict(
            os.environ,
            {
                "CARGO_TARGET_DIR": str(target),
                runner_var: shlex.join(
                    [sys.executable, str(runner), str(self.other), str(target)]
                ),
            },
        ):
            self.assert_pinned_build(self.root)
