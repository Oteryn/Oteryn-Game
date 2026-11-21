"""The protected cache writer must finish and publish only a bounded archive."""

import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import textwrap
import unittest


WORKFLOW = Path(__file__).resolve().parents[2] / ".github/workflows/rust.yml"
BUDGET = "4294967296"
CACHE_PATHS = [
    "~/.cargo/registry/index/", "~/.cargo/registry/cache/", "~/.cargo/git/db/", "target/",
]


class ProtectedMainCache(unittest.TestCase):
    def setUp(self):
        self.source = WORKFLOW.read_text(encoding="utf-8")
        self.linux = self.source.split("\n  linux:\n", 1)[1]
        self.steps = re.split(r"(?m)^      - name: ", self.linux)[1:]
        self.trim = next(step for step in self.steps if "normalize_cargo_mtimes.py --record" in step)
        self.save = next(step for step in self.steps if "uses: actions/cache/save@" in step)

    def run_trim(self, fail_phase=""):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "target").mkdir()
            binaries = root / "bin"
            binaries.mkdir()
            stub = f"#!{sys.executable}\n" + textwrap.dedent('''\
                import json, os, sys
                from pathlib import Path
                args = sys.argv[1:]
                phase = ("record" if args[0].endswith("normalize_cargo_mtimes.py") else
                         "check" if "--check-only" in args else "prune")
                with Path(os.environ["CALLS"]).open("a") as stream:
                    stream.write(json.dumps({"phase": phase, "args": args}) + "\\n")
                if phase == os.environ["FAIL_PHASE"]:
                    sys.exit(42)
                if "--metrics" in args:
                    Path(args[args.index("--metrics") + 1]).write_text("{}\\n")
            ''')
            for name in ("python", "python3"):
                path = binaries / name
                path.write_text(stub, encoding="utf-8")
                path.chmod(0o700)
            env = {
                "PATH": str(binaries) + os.pathsep + os.defpath,
                "HOME": str(root / "home"), "RUNNER_TEMP": str(root),
                # The cache action uploads ~/.cargo, regardless of this override.
                "CARGO_HOME": str(root / "other-cargo"),
                "GITHUB_STEP_SUMMARY": str(root / "summary"),
                "CALLS": str(root / "calls"), "FAIL_PHASE": fail_phase,
            }
            script = textwrap.dedent(self.trim.split("        run: |\n", 1)[1])
            result = subprocess.run(["bash", "-c", script], cwd=root, env=env,
                                    capture_output=True, text=True, timeout=10, check=False)
            calls = [json.loads(line) for line in (root / "calls").read_text().splitlines()]
            return result, calls, root

    def test_budget_includes_the_final_source_snapshot_and_exact_uploaded_roots(self):
        result, calls, root = self.run_trim()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual([call["phase"] for call in calls], ["prune", "record", "check"])
        for call in (calls[0], calls[2]):
            args = call["args"]
            self.assertEqual(args[0], "tools/repository/trim_cargo_cache.py")
            for flag, value in (("--workspace", "."), ("--target", "target"),
                                ("--cargo-home", str(root / "home/.cargo")), ("--max-bytes", BUDGET)):
                self.assertEqual(args[args.index(flag) + 1], value)
            self.assertEqual(Path(args[args.index("--metrics") + 1]).parent, root)
        self.assertNotIn("--check-only", calls[0]["args"])
        self.assertIn("--check-only", calls[2]["args"])
        self.assertEqual(calls[1]["args"], ["tools/repository/normalize_cargo_mtimes.py", "--record"])

    def test_every_prepublication_failure_stops_the_remaining_commands(self):
        phases = ["prune", "record", "check"]
        for index, phase in enumerate(phases):
            with self.subTest(phase=phase):
                result, calls, _ = self.run_trim(phase)
                self.assertEqual(result.returncode, 42, result.stderr)
                self.assertEqual([call["phase"] for call in calls], phases[:index + 1])

    def test_publication_remains_protected_and_preserves_consumer_paths_and_keys(self):
        fence = "        if: github.ref == 'refs/heads/main' && github.ref_protected\n"
        self.assertIn(fence, self.trim)
        self.assertIn(fence, self.save)
        self.assertNotIn("continue-on-error", self.linux)
        self.assertNotIn("always()", self.trim + self.save)
        self.assertLess(self.steps.index(self.trim), self.steps.index(self.save))
        self.assertEqual(self.save, self.steps[-1])
        cached = [step for step in self.steps if "uses: actions/cache/" in step]
        self.assertEqual(len(cached), 2)
        for step in cached:
            paths = step.split("          path: |\n", 1)[1].split("          key:", 1)[0]
            self.assertEqual([line.strip() for line in paths.splitlines()], CACHE_PATHS)
            self.assertIn("key: rust-linux-v2-1.100.0-${{ hashFiles('Cargo.lock') }}-${{ github.sha }}\n", step)

    def test_frequent_main_pushes_preserve_the_active_warmup(self):
        self.assertIn("concurrency:\n  group: rust-protected-main-cache-${{ github.ref }}\n"
                      "  cancel-in-progress: false\n", self.source)
        self.assertIn("  workflow_dispatch:\n", self.source)
        self.assertIn("    needs: lanes\n", self.linux)


if __name__ == "__main__":
    unittest.main()
