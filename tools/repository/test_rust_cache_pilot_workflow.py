#!/usr/bin/env python3
"""Exercise the pilot's actual source-binding step against a small Git history."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import textwrap
import unittest


ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github/workflows/rust-cache-pilot.yml"
MEASURE_STEP = "Measure compilation with isolated cold and warm targets and optional edits"


class RustCachePilotWorkflowTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name) / "checkout"
        self.root.mkdir()
        self.runner_temp = Path(temp.name) / "runner"
        self.runner_temp.mkdir()
        self.controller_file = self.root / "tools/repository/benchmark_rust_cache.py"
        self.controller_file.parent.mkdir(parents=True)
        # This filename is a Git pathspec expression when it is interpreted as magic.
        self.edit_path = ":(glob)*.rs"
        (self.root / self.edit_path).write_text("pub fn answer() -> u32 { 1 }\n")
        (self.root / "other.rs").write_text("pub fn other() -> u32 { 1 }\n")
        (self.root / "unchanged.rs").write_text("pub fn unchanged() -> u32 { 1 }\n")
        (self.root / "README.md").write_text("base\n")
        (self.root / "link.rs").symlink_to("other.rs")
        self.controller_file.write_text("historical controller\n")
        self.git("init", "--quiet")
        self.base = self.commit("base")

        (self.root / self.edit_path).write_text("pub fn answer() -> u32 { 2 }\n")
        (self.root / "other.rs").write_text("pub fn other() -> u32 { 3 }\n")
        (self.root / "README.md").write_text("edited\n")
        self.edited = self.commit("edited")

        self.controller_file.write_text("trusted controller\n")
        self.controller = self.commit("trusted controller")

    def git(self, *args, **kwargs):
        return subprocess.check_output(["git", "-c", "core.hooksPath=/dev/null", "-C", str(self.root), *args],
                                       text=True, **kwargs).strip()

    def commit(self, message):
        self.git("add", "--all")
        self.git("-c", "user.name=Fixture", "-c", "user.email=fixture@example.test",
                 "commit", "--quiet", "-m", message)
        return self.git("rev-parse", "HEAD")

    def run_step(self, name, **env):
        steps = WORKFLOW.read_text().split(f"      - name: {name}\n", 1)[1]
        step = steps.split("\n      - name:", 1)[0]
        script = textwrap.dedent(step.split("        run: |\n", 1)[1])
        return subprocess.run(["bash", "-c", script], cwd=self.root,
                              env={**os.environ, "RUNNER_TEMP": str(self.runner_temp), **env},
                              text=True, capture_output=True, timeout=10)

    def bind(self, source="", edited="", path=""):
        return self.run_step("Bind benchmark to accepted main source", CONTROLLER_SHA=self.controller,
                             REQUESTED_SHA=source, EDIT_SHA=edited, EDIT_PATH=path)

    def measure_arguments(self, scope, edited=""):
        self.controller_file.write_text("import json\nimport sys\nprint(json.dumps(sys.argv[1:]))\n")
        result = self.run_step(MEASURE_STEP, BUILD_SCOPE=scope, EDIT_SHA=edited)
        self.assertEqual(result.returncode, 0, result.stderr)
        return json.loads(result.stdout)

    def test_accepted_edit_is_exactly_one_literal_path(self):
        result = self.bind(self.base, self.edited, self.edit_path)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.git("rev-parse", "HEAD"), self.base)
        self.assertEqual(self.controller_file.read_text(), "trusted controller\n")
        controls = self.runner_temp / "pilot-controller"
        patch = controls / "source-edit.patch"
        self.assertTrue(patch.is_file())
        self.assertEqual(patch.read_text().strip(), self.git("--literal-pathspecs", "diff", "--full-index",
                                                           self.base, self.edited, "--", self.edit_path))
        self.git("apply", str(patch))
        self.assertEqual((self.root / self.edit_path).read_text(), "pub fn answer() -> u32 { 2 }\n")
        self.assertEqual((self.root / "other.rs").read_text(), "pub fn other() -> u32 { 1 }\n")
        provenance = json.loads((controls / "source-edit-provenance.json").read_text())
        self.assertEqual(provenance, {"source_sha": self.base, "edit_sha": self.edited, "edit_path": self.edit_path})
        args = self.measure_arguments("server-library", self.edited)
        self.assertEqual(args[-4:], ["--package", "oteryn-game-server", "--edit-patch", str(patch)])
        attached = self.run_step("Attach source edit provenance")
        self.assertEqual(attached.returncode, 0, attached.stderr)
        result_path = self.runner_temp / "rust-cache-results/source-edit-provenance.json"
        self.assertEqual(json.loads(result_path.read_text()), provenance)
        patch.unlink()
        missing = self.run_step(MEASURE_STEP, BUILD_SCOPE="server-library", EDIT_SHA=self.edited)
        self.assertNotEqual(missing.returncode, 0, missing.stdout)

    def test_bad_edit_input_is_rejected_before_checkout(self):
        tree = self.git("rev-parse", "HEAD^{tree}")
        unaccepted = self.git("-c", "user.name=Fixture", "-c", "user.email=fixture@example.test",
                              "commit-tree", tree, "-p", self.controller, "-m", "unaccepted")
        for source, edited, path in (
            (self.base, self.edited, ""),
            (self.base, "", self.edit_path),
            (self.base, "HEAD", self.edit_path),
            (self.base, unaccepted, self.edit_path),
            (self.controller, self.edited, self.edit_path),
            (self.base, self.edited, "README.md"),
            (self.base, self.edited, "link.rs"),
            (self.base, self.edited, "missing.rs"),
            (self.base, self.edited, "unchanged.rs"),
        ):
            with self.subTest(source=source, edited=edited, path=path):
                result = self.bind(source, edited, path)
                self.assertNotEqual(result.returncode, 0, result.stdout)
                self.assertEqual(self.git("rev-parse", "HEAD"), self.controller)
                self.assertFalse((self.runner_temp / "pilot-controller/source-edit.patch").exists())

    def test_edits_are_optional_and_source_sha_retains_its_meaning(self):
        result = self.bind(self.base)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.git("rev-parse", "HEAD"), self.base)
        self.assertEqual(self.controller_file.read_text(), "trusted controller\n")
        self.assertFalse((self.runner_temp / "pilot-controller/source-edit.patch").exists())
        args = self.measure_arguments("workspace")
        self.assertNotIn("--package", args)
        self.assertNotIn("--edit-patch", args)


if __name__ == "__main__":
    unittest.main()
